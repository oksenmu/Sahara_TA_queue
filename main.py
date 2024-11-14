from http.server import BaseHTTPRequestHandler, HTTPServer
from http.cookies import SimpleCookie
from base64 import b64encode
import json
import os
import sys
import secrets

#hostName = "10.24.12.221" 
hostName = "localhost"
serverPort = 8080

users = []
queue = []

def add_user():
    print("Adding user")
    id = b64encode(secrets.token_bytes(32)).decode()
    cookie = SimpleCookie()
    cookie["id"] = id  # Cookie name and value
    cookie["id"]["path"] = "/"  # Path where the cookie is accessible
    cookie["id"]["httponly"] = True
    cookie["id"]["SameSite"] = "Strict"
    users.append(Student(id))
    return cookie

class Student():

    def __init__(self, id):
        self.id = id
        self.ta = False
        self.table_id = None
        self.task = None
        self.helped_by = None
        self.user_name = None


class MyServer(BaseHTTPRequestHandler):

    def cookie(self):
        if "Cookie" in self.headers:
            cookie = SimpleCookie(self.headers["Cookie"])
            if cookie.get("id") is None or cookie.get("id").value not in [u.id for u in users]:
                cookie = add_user()
                self.send_header("Set-Cookie", cookie.output(header="", sep=""))
        else:
            cookie = add_user()
            self.send_header("Set-Cookie", cookie.output(header="", sep=""))

    def serve(self, path, code, t):
        with open(path, "rb") as f:
            data = f.read()
        self.send_response(code)
        if code == 200 and t == "text/html":
            self.cookie()
        self.send_header("Content-type", t)
        self.end_headers()
        self.wfile.write(data)
        pass

    def serve_html(self, path):
        self.serve(path, 200, "text/html")

    def serve_404(self):
        self.serve("./pages/404.html", 404, "text/html")

    def serve_css(self, path):
        self.serve(path, 200, "text/css")

    def serve_js(self, path):
        self.serve(path, 200, "text/javascript")

    def serve_admin_page(self):
        cookie = SimpleCookie(self.headers["Cookie"])
        if cookie.get("id") == None:
            self.serve("./pages/403.html", 403, "text/html")
            return
        id = cookie.get("id").value
        user = None
        for u in users:
            if u.id == id:
                user = u
                break
        if user is None or not user.ta:
            self.serve("./pages/403.html", 403, "text/html")
        else:
            self.serve_html("./pages/admin.html")

    def get_user(self):
        cookie = SimpleCookie(self.headers["Cookie"])
        if cookie.get("id") == None:
            self.send_response(400)
            self.end_headers()
            self.wfile.write(json.dumps({"error" : "Missing cookie", "data" : ""}).encode())
            return
        id = cookie.get("id").value
        user = None
        for u in users:
            if u.id == id:
                user = u
                break
        if user == None:
            self.send_response(400)
            self.end_headers()
            self.wfile.write(json.dumps({"error" : "Missing cookie", "data" : ""}).encode())
            return
        return user

    def get_queue(self):
        user = self.get_user()
        if user is None:
            return
        if not user.ta:
            self.serve("./pages/403.html", 403, "text/html")
            return
        self.send_response(200)
        self.send_header("Content-type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps({"error" : "", "data" : [{"table_id" : u.table_id, "task" : u.task, "helped_by" : u.helped_by , "id" : u.id } for u in queue]}).encode())
        
    
    def promote_student(self):
        user = self.get_user()
        if user is None:
            return
        try:
            token = self.rfile.read(int(self.headers['Content-Length'])).decode()
            if token != os.environ["TA_TOKEN"]:
                self.send_response(403)
                self.send_header("Content-type", "application/json")
                self.end_headers()
                self.wfile.write(json.dumps({"error" : "Cannot validate token", "data" : ""}).encode())
                return
            user.ta = True
            user.name = "Teaching Assistant"
            self.send_response(200)
            self.send_header("Content-type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"error" : "", "data" : "You are now promoted to TA"}).encode())
        except Exception as e:
            self.send_response(400)
            self.send_header("Content-type", "application/json")
            self.end_headers()
            print(f"Error {e}")
            self.wfile.write(json.dumps({"error" : "Invalid token format", "data" : ""}).encode())

    def request_help(self):
        user = self.get_user()
        if user is None:
            return
        if user in queue:
            self.send_response(200)
            self.send_header("Content-type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"error" : "Alredy in queue", "data" : ""}).encode())
            return
        try:
            info = json.loads(self.rfile.read(int(self.headers['Content-Length'])).decode())
            user.table_id = info["table_id"]
            user.task = info["task"]
        except:
            self.send_response(400)
            self.send_header("Content-type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"error" : "Invalid JSON", "data" : ""}).encode())
            return
        queue.append(user)
        self.send_response(200)
        self.send_header("Content-type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps({"error" : "", "data" : "ok!"}).encode())

    def uppdate_name(self):
        user = self.get_user()
        if user is None:
            return
        try:
            name = self.rfile.read(int(self.headers['Content-Length'])).decode()
            user.name = name
        except:
            self.send_response(400)
            self.send_header("Content-type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"error" : "Invalid JSON", "data" : ""}).encode())
            return
        self.send_response(200)
        self.send_header("Content-type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps({"error" : "", "data" : "ok!"}).encode())

    def get_index(self):
        user = self.get_user()
        if user is None:
            return
        if user.helped_by is not None:
            self.send_response(200)
            self.send_header("Content-type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"error" : "", "data" : {"status": "Getting help", "index" : 0, "helped_by" : user.helped_by}}).encode())
            return
        if user not in queue:
            self.send_response(200)
            self.send_header("Content-type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"error" : "", "data" : {"status": "Not in queue", "index" : 0, "helped_by" : None}}).encode())
            return
        queue_real = [u for u in queue if u.helped_by is None]
        self.send_response(200)
        self.send_header("Content-type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps({"error" : "", "data" : {"status": "In queue", "index" : queue_real.index(user)+1, "helped_by" : None}}).encode())

    def get_name(self):
        user = self.get_user()
        if user is None:
            return
        self.send_response(200)
        self.send_header("Content-type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps({"error" : "", "data" : user.name}).encode())

    def get_stats(self):
        data = {}
        if os.path.exists("stats/ta.json"):
            with open("stats/ta.json" , "rb") as f:
                data = json.loads(f.read().strip())
        self.send_response(200)
        self.send_header("Content-type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps({"error" : "", "data" : data}).encode())

    def remove_from_queue(self):
        user = self.get_user()
        if user is None:
            return
        if not user.ta:
            self.send_response(403)
            self.send_header("Content-type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"error" : "You are not admin", "data" : ""}).encode())
            return
        id = self.rfile.read(int(self.headers['Content-Length'])).decode()
        student = None
        for u in queue:
            if u.id == id:
                student = u
                break
        if student == None:
            self.send_response(400)
            self.send_header("Content-type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"error" : "Person is not in queue", "data" : ""}).encode())
            return
        student.helped_by = None
        student.task = None
        queue.remove(student)
        self.send_response(200)
        self.send_header("Content-type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps({"error" : "", "data" : "ok!"}).encode())

    def increase_stats(self, name):
        data = {}
        if os.path.exists("stats/ta.json"):
            with open("stats/ta.json" , "rb") as f:
                data = json.loads(f.read().strip())
        if name in data:
            data[name] = data[name] + 1
        else:
            data[name] = 1
        with open("stats/ta.json", "w") as f:
            f.write(json.dumps(data))


    def help(self):
        user = self.get_user()
        if user is None:
            return
        if not user.ta:
            self.send_response(403)
            self.send_header("Content-type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"error" : "You are not admin", "data" : ""}).encode())
            return
        id = self.rfile.read(int(self.headers['Content-Length'])).decode()
        student = None
        for u in queue:
            if u.id == id:
                student = u
                break
        if student == None:
            self.send_response(400)
            self.send_header("Content-type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"error" : "Person is not in queue", "data" : ""}).encode())
            return
        student.helped_by = user.name
        self.increase_stats(user.name)
        self.send_response(200)
        self.send_header("Content-type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps({"error" : "", "data" : "ok!"}).encode())
        
        

    def do_GET(self):
        match self.path:
            case "/":
                self.serve_html("./pages/index.html")
            case "/admin":
                self.serve_admin_page()
            case "/ta":
                self.serve_html("./pages/ta.html")
            case "/api/queue":
                self.get_queue()
            case "/api/my_index":
                self.get_index()
            case "/api/my_name":
                self.get_name()
            case "/api/stats":
                self.get_stats()
            case "/index.js":
                self.serve_js("./pages/index.js")
            case "/admin.js":
                self.serve_js("./pages/admin.js")
            case "/404.css":
                self.serve_css("./pages/404.css")
            case "/403.css":
                self.serve_css("./pages/403.css")
            case "/styles.css":
                self.serve_css("./pages/styles.css")
            case _:
                self.serve_404()

    def do_POST(self):
        match self.path:
            case "/ta":
                self.promote_student()
            case "/request_help":
                self.request_help()
            case "/remove_from_queue":
                self.remove_from_queue()
            case "/update_name":
                self.uppdate_name()
            case "/help":
                self.help()
            case _:
                self.send_response(404)
                self.send_header("Content-type", "application/json")
                self.end_headers()
                self.wfile.write(json.dumps({"error" : "Invalid endpoint", "data" : ""}).encode())
                return


if __name__ == "__main__":        
    webServer = HTTPServer((hostName, serverPort), MyServer)
    print("Server started http://%s:%s" % (hostName, serverPort))
    if "TA_TOKEN" not in os.environ or os.environ["TA_TOKEN"] == "PLACEHOLDER":
        print("Please configure a secure TA_TOKEN in the .env file")
        print(f"Sugested token = {b64encode(secrets.token_bytes(32)).decode()}")
        sys.exit()

    try:
        webServer.serve_forever()
    except KeyboardInterrupt:
        pass

    webServer.server_close()
    print("Server stopped.")
