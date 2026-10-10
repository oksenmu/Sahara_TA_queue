TABLE_NUMBER = Math.max(...[
    ...document
    .querySelectorAll('[data-table-number]')
    .values()
    .map(e=>Number(e.getAttribute("data-table-number")))
]);
var ROOM = document.getElementById("room-layout").getAttribute("data-room-name")
let queue = []

function removeUser(id) {
    data = {command: "remove", argument: parseInt(id), "value": ""}
    socket.send(JSON.stringify(data));
}

function helpUser(id) {
    data = {command: "help", argument: parseInt(id), "value": ""}
    socket.send(JSON.stringify(data));
}

function resetTables(){
    for (let i = 1; i <= TABLE_NUMBER; i++) {
        table = document.getElementById(i)
        table.classList = ["table"]
        table.innerText = ""
    }
}

function updateTable(id, room, index){
    if (room != ROOM){
        return;
    }
    table = document.getElementById(id)
    if (table.innerText != "") return
    table.innerText = index
    if (index == 0){
        table.classList.add("helping")
    }
    else if (index == 1){
        table.classList.add("first")
    }
    else{
        table.classList.add("in-queue")
    }
}


function updateName() {
    fetch("/api/name", {
        method: "GET",
        headers: {'Content-Type': 'text/plain'},
    }).then(res => 
        res.json()
    ).then(data=>{
        if (data["error"] == ""){
            document.getElementById("name-info").innerText = `Name: ${data["data"]}`;
        }
        else{
            console.log(data["error"])
        }
    });
}


function make_queue(queue){
        resetTables()
        table = document.getElementById("queue")
        while(table.firstChild){
            table.removeChild(table.firstChild)
        }
        tbody = document.createElement("tbody")
        table.appendChild(tbody)
        row = document.createElement("tr")
        tbody.appendChild(row)
        th = document.createElement("th")
        th.innerText = "Index"
        row.appendChild(th)
        th = document.createElement("th")
        th.innerText = "Room"
        row.appendChild(th)
        th = document.createElement("th")
        th.innerText = "Needs help with"
        row.appendChild(th)
        th = document.createElement("th")
        th.innerText = "Help"
        row.appendChild(th)
        th = document.createElement("th")
        th.innerText = "Remove from queue"
        row.appendChild(th)
        offset = 0
        max_index = 0
        for (let i = 0; i < queue.length; i++) {
            data_element = queue[i]
            row = document.createElement("tr")
            row.id = data_element["id"]
            if(data_element["index"] > 0){
                tbody.appendChild(row)
                index = i+1-offset
            }
            else{
                row.classList.add("highlight-blue")
                if (tbody.children.length > 1){
                    tbody.insertBefore(row, tbody.children[1])
                }
                else{
                    tbody.appendChild(row)
                }
                index = 0
                offset += 1
            }
            td = document.createElement("td")
            td.innerText = index
            row.appendChild(td)
            td = document.createElement("td")
            if (data_element["room"] == ROOM) {
                td.innerText = data_element["room"]
            } else {
                a = document.createElement("a")
                a.href = data_element["room"]
                a.innerText = data_element["room"]
                td.appendChild(a)
            }
            row.appendChild(td)
            td = document.createElement("td")
            if ("helped_by" in data_element){
                td.innerText = `${data_element["task"]} (Helped by : ${data_element["helped_by"]})`
            } else {
                td.innerText = data_element["task"]
            }
            row.appendChild(td)
            td = document.createElement("td")
            button = document.createElement("button")
            button.innerText = "Help"
            button.onclick = (event) => helpUser(event.target.parentNode.parentNode.id)
            td.appendChild(button)
            row.appendChild(td)
            td = document.createElement("td")
            button = document.createElement("button")
            button.innerText = "Remove"
            button.onclick = (event) => removeUser(event.target.parentNode.parentNode.id)
            td.appendChild(button)
            row.appendChild(td)
            updateTable(data_element["table_number"], data_element["room"], index)
            if (index > max_index) max_index = index
        }
        document.getElementById("title").innerText = `(${offset}/${max_index+offset}) TA queue admin`
}

function sendUpdateName(){
    data = {command: "set_name", argument: 0, "value": document.getElementById("name").value}
    socket.send(JSON.stringify(data));
    fetch("/api/name", {
        method: "POST",
        headers: {'Content-Type': 'text/plain'}, 
        body: document.getElementById("name").value
    }).then(res => 
        res.json()
    ).then(data=>{
        if (data["error"] != ""){
            document.getElementById("warning").innerText = data["error"];
            setTimeout(() => {
                document.getElementById("warning").innerHTML = ""
            }, 7000);
        }
        else{
            document.getElementById("name-info").innerText = document.getElementById("name").value
        }
    });
}


const socket = new WebSocket('/ta_ws');
socket.onmessage = function (event) {
    const data = JSON.parse(event.data);
    if (data["message_type"] == "Queue"){
        let new_queue = JSON.parse(data["data"])
        if (new_queue.map(e=>e.id).filter(e=>!queue.map(f=>f.id).includes(e)).length > 0){
            console.log("Notif sent")
            const joined = new Notification("Student joined the queue");
            document.addEventListener("visibilitychange", () => {
              if (document.visibilityState === "visible") {
                joined.close();
              }
            });
        }
        queue = new_queue
        make_queue(queue)
        console.log(queue)
    }
    else{
        console.log(data["error"])
    }
}
socket.onopen = function () {
    // Send a message to the WebSocket server
    //socket.send(document.cookie);
    data = {command: "get_queue", argument: 0, "value": ""}
    socket.send(JSON.stringify(data));
};

document.getElementById('update_name').addEventListener('click', () => {
    sendUpdateName()
});

document.getElementById('name').addEventListener('keypress', (event) => {
    if(event.key === "Enter") sendUpdateName()
});

Notification.requestPermission().then((result) => {
  console.log(result);
});


updateName()

