var input = document.getElementById("token");
var has_rooms = false;
var has_config = false;
var rooms = [];
var config = {};

function build_ui(){
    if (!(has_rooms && has_config)){
        return;
    }
    index_room = document.getElementById("index-room")
    enabled_rooms = document.getElementById("enabled-rooms")
    rooms.forEach(room=>{
        option = document.createElement("option")
        option.value = room
        option.innerText = room
        index_room.appendChild(option)

        input = document.createElement("input")
        input.type = "checkbox"
        input.value = room
        enabled_rooms.appendChild(input)
        if (config["rooms"].includes(room)){
            input.checked = true;
        }
        label = document.createElement("label")
        label.innerText = room
        enabled_rooms.appendChild(label)
        enabled_rooms.appendChild(document.createElement("br"))
    })
    index_room.value = config["main_room"]
}

function submit(){
    if (!(has_rooms && has_config)){
        return;
    }
    main_room = document.getElementById("index-room").value
    rooms = [...document.querySelectorAll('#enabled-rooms input[type="checkbox"]').values().filter(e=>e.checked).map(e=>e.value)];
    fetch("/api/settings", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({main_room: main_room, rooms:rooms})
    })
        .then((res) =>{
            if (!res.ok) {
                throw new Error(`HTTP error: ${res.status}`);
            }
            return res.json();
        })
        .then((data) => {
            document.getElementById("status").innerText = "Uppdated settings";
        })
        .catch((err) => document.getElementById("status").innerText =err);
}

fetch("/api/rooms", {
    method: "GET",
    headers: {'Content-Type': 'text/json'}, 
}).then(res => 
    res.json()
).then(data=>{
    rooms = data;
    has_rooms = true;
    build_ui();
});

fetch("/api/settings", {
    method: "GET",
    headers: {'Content-Type': 'text/json'}, 
}).then(res => 
    res.json()
).then(data=>{
    config = data;
    has_config = true;
    build_ui();
});


