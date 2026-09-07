function removeUser(id) {
    data = {command: "remove", argument: parseInt(id)}
    socket.send(JSON.stringify(data));
}

function helpUser(id) {
    data = {command: "help", argument: parseInt(id)}
    socket.send(JSON.stringify(data));
}

function resetTables(){
    for (let i = 1; i <= 36; i++) {
        table = document.getElementById(i)
        table.classList = ["table"]        
        table.innerText = ""
    }
}

function updateTable(id, index){
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

document.getElementById('update_name').addEventListener('click', () => {
    sendUpdateName()
});

document.getElementById('name').addEventListener('keypress', (event) => {
    if(event.key === "Enter") sendUpdateName()
});

function sendUpdateName(){
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

const socket = new WebSocket('ws://'+ window.location.hostname + ':27890/ta');
socket.onopen = function () {
    // Send a message to the WebSocket server
    //socket.send(document.cookie);
    data = {command: "get_queue", argument: 0}
    socket.send(JSON.stringify(data));
};

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
        th.innerText = "Table ID"
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
            row.id = data_element["table_number"]
            if(data_element["index"] !== 0){
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
            td.innerText = data_element["table_number"]
            row.appendChild(td)
            td = document.createElement("td")
            td.innerText = data_element["task"]
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
            updateTable(data_element["table_number"], index)
            if (index > max_index) max_index = index
        }
        document.getElementById("title").innerText = `(${offset}/${max_index+offset}) TA queue admin`

}

socket.onmessage = function (event) {
    const data = JSON.parse(event.data);
    if (data["message_type"] == "Queue"){
        const queue = JSON.parse(data["data"])
        make_queue(queue)
        console.log(queue)
    }
    else{
        console.log(data["error"])
    }
}

updateName()
