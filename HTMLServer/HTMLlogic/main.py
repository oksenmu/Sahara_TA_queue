from flask_jwt_extended import JWTManager, create_access_token, get_jwt_identity, jwt_required
from flask import Flask, request, render_template, make_response, jsonify

import datetime
import json
import os
import logging
import uuid
import secrets

app = Flask(__name__)
app.config["JWT_TOKEN_LOCATION"] = ["cookies"]
app.config["JWT_COOKIE_CSRF_PROTECT"] = False  # Disable CSRF protection for testing
app.config['JWT_SECRET_KEY'] = os.environ.get('JWT_PASSWORD', 'supersecretkey')  # Change this to a secure key in production
app.config['JWT_ACCESS_TOKEN_EXPIRES'] = datetime.timedelta(days=1)

app.logger.setLevel(logging.DEBUG)  # Ensure logging level is set to DEBUG
logging.basicConfig(level=logging.DEBUG)  # Configure logging


jwt = JWTManager(app)

def generate_deafault_user():
    return {
        "ta": False,
        "table_number": 0,
        "name": "Teaching assistant",
        "task": "",
        "helped_by": "",
        "id": secrets.randbelow(2**128)
    }

def set_jwt_cookie(response, user_data):
    access_token = create_access_token(identity=json.dumps(user_data))
    response.set_cookie("access_token_cookie", access_token, httponly=True, samesite="strict")
    return response

@app.errorhandler(404)
def not_found_error(_):
    return render_template("404.html"), 404

@app.route('/', methods=['GET'])
def home():
    app.logger.debug(app.config['JWT_SECRET_KEY'])
    if not request.cookies.get("access_token_cookie"):
        user_data = generate_deafault_user()
        response = make_response(render_template("index.html"))
        return set_jwt_cookie(response, user_data)
    return render_template("index.html")

@app.route('/ta', methods=['GET'])
def ta():
    if not request.cookies.get("access_token_cookie"):
        user_data = generate_deafault_user()
        response = make_response(render_template("ta.html"))
        return set_jwt_cookie(response, user_data)
    return render_template("ta.html")

@app.route('/ta', methods=['POST'])
@jwt_required(locations=["cookies"])
def update_token():
    password = request.data.decode()
    
    if password == os.environ.get("TA_TOKEN", "TestToken"):
        user = json.loads(get_jwt_identity())
        user["ta"] = True  # Update user data
        response = make_response(jsonify({"error": ""}))
        return set_jwt_cookie(response, user)
    else:
        return jsonify({"error": "Wrong token"}), 403


@app.route('/admin', methods=['GET'])
@jwt_required(locations=["cookies"])
def admin():
    user = json.loads(get_jwt_identity())
    if not user.get("ta", False):
        return render_template("403.html"), 403
    return render_template("admin.html")

@app.route('/api/table_number', methods=['GET'])
@jwt_required(locations=["cookies"])
def get_table_number():
    try:
        user = json.loads(get_jwt_identity())
        return jsonify({"error": "", "data": user["table_number"]})
    except Exception as e:
        return jsonify({"error": str(e), "data": {}}), 500

@app.route('/api/table_number', methods=['POST'])
@jwt_required(locations=["cookies"])
def update_table_number():
    table_number = int(request.data.decode())
    if 1 <= table_number <= 30:
        user = json.loads(get_jwt_identity())
        user["table_number"] = table_number
        response = make_response(jsonify({"error": ""}))
        return set_jwt_cookie(response, user)
    return make_response(jsonify({"error": "Not valid table_number"}))

@app.route('/api/name', methods=['GET'])
@jwt_required(locations=["cookies"])
def get_name():
    try:
        user = json.loads(get_jwt_identity())
        return jsonify({"error": "", "data": user["name"]})
    except Exception as e:
        return jsonify({"error": str(e), "data": {}}), 500

@app.route('/api/name', methods=['POST'])
@jwt_required(locations=["cookies"])
def update_name():
    name = request.data.decode()
    user = json.loads(get_jwt_identity())
    user["name"] = name
    response = make_response(jsonify({"error": ""}))
    return set_jwt_cookie(response, user)

@app.route('/api/task', methods=['GET'])
@jwt_required(locations=["cookies"])
def get_task():
    try:
        user = json.loads(get_jwt_identity())
        return jsonify({"error": "", "data": user["task"]})
    except Exception as e:
        return jsonify({"error": str(e), "data": {}}), 500

@app.route('/api/task', methods=['POST'])
@jwt_required(locations=["cookies"])
def update_task():
    task = request.data.decode()
    user = json.loads(get_jwt_identity())
    user["task"] = task
    response = make_response(jsonify({"error": ""}))
    return set_jwt_cookie(response, user)
