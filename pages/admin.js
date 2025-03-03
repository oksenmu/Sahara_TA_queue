function removeUser(id) {
    fetch("/remove_from_queue", {
        method: "POST",
        headers: {'Content-Type': 'text/plain'}, 
        body: id }).then(res => res.json()
    ).then(data=>{
        updateQueue()
        if (data["error"] != ""){
            console.log(data["error"])
        }
    });
}

function helpUser(id) {
    fetch("/help", {
        method: "POST",
        headers: {'Content-Type': 'text/plain'}, 
        body: id
    }).then(res => 
        res.json()
    ).then(data=>{
       updateQueue()
        if (data["error"] != ""){
            console.log(data["error"])
        }
    });
}

function resetTables(){
    for (let i = 1; i <= 30; i++) {
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

function updateQueue() {
    fetch("/api/queue", {
        method: "GET",
        headers: {'Content-Type': 'text/plain'}, 
    }).then(res => 
        res.json()
    ).then(data=>{
        if (data["error"] == ""){
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
            for (let i = 0; i < data["data"].length; i++) {
                data_element = data["data"][i]
                row = document.createElement("tr")
                row.id = data_element["id"]
                if(data_element["helped_by"] == null){
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
                td.innerText = data_element["table_id"]
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
                updateTable(data_element["table_id"], index)
                if (index > max_index) max_index = index
            }
            document.getElementById("title").innerText = `(${offset}/${max_index+offset}) TA queue admin`
        }
        else{
            console.log(data["error"])
        }
    });
    
}


document.getElementById('update_name').addEventListener('click', () => {
    sendUpdateName()
});

document.getElementById('name').addEventListener('keypress', (event) => {
    if(event.key === "Enter") sendUpdateName()
});

function sendUpdateName(){
    fetch("/update_name", {
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
            updateName()
        }
    });

}

function updateName() {
    fetch("/api/my_name", {
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

const socket = new WebSocket('ws://'+ window.location.hostname + ':27891');
socket.onopen = function () {
    // Send a message to the WebSocket server
    socket.send(document.cookie);
};

socket.onmessage = function (event) {
    const data = JSON.parse(event.data);
    if (data["error"] == ""){
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
        for (let i = 0; i < data["data"].length; i++) {
            data_element = data["data"][i]
            row = document.createElement("tr")
            row.id = data_element["id"]
            if(data_element["helped_by"] == null){
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
            td.innerText = data_element["table_id"]
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
            updateTable(data_element["table_id"], index)
            if (index > max_index) max_index = index
        }
        document.getElementById("title").innerText = `(${offset}/${max_index+offset}) TA queue admin`
    }
    else{
        console.log(data["error"])
    }
}

updateName()
