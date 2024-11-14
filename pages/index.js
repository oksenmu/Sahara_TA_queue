var chosen = "?"

document.querySelectorAll('.table').forEach(table => {
    table.addEventListener('click', () => {
        if(chosen != "?"){
            document.getElementById(chosen).classList.remove("chosen-table");
        }
        table.classList.add("chosen-table");
        chosen = table.getAttribute('data-table-number');
        document.getElementById("button").innerText = "Call TA " + chosen
    });
});

document.getElementById('button').addEventListener('click', () => {
    requestHelp()
});

document.getElementById('oppgave').addEventListener('keypress', (event) => {
    if(event.key === "Enter") requestHelp()
});

function requestHelp(){
    if (chosen != "?"){
        fetch("/request_help", {
            method: "POST",
            headers: {'Content-Type': 'application/json'}, 
            body: JSON.stringify({table_id:chosen, task:document.getElementById("oppgave").value})
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
                updatePosition()
            }
        });
    }
    else{
        document.getElementById("warning").innerText = "Please select your table";
        setTimeout(() => {
            document.getElementById("warning").innerHTML = ""
        }, 10000);
    }

}

function updatePosition() {
    fetch("/api/my_index", {
        method: "GET",
        headers: {'Content-Type': 'text/plain'}, 
    }).then(res => 
        res.json()
    ).then(data=>{
        if (data["error"] == "" && data["data"]["status"]=="In queue"){
            document.getElementById("queue-info").innerText = `You are number ${data["data"]["index"]} in the queue`;
            document.getElementById("title").innerText = `TA queue q=${data["data"]["index"]}`
        }
        else if(data["error"] == "" && data["data"]["status"]=="Getting help"){
            document.getElementById("queue-info").innerText = `You are now getting help from ${data["data"]["helped_by"]}`;
            document.getElementById("title").innerText = `TA queue q=0`
        }
        else{
            document.getElementById("queue-info").innerText = ""
            document.getElementById("title").innerText = `TA queue`
        }
    });
}

setInterval(() => {
    updatePosition()
}, 3000);

updatePosition()