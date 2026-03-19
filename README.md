# Sahara TA Queue

**The map has not been updated since Sahara was renovated.**

This is a queue system for TAs using the Sahara lab at IIK NTNU.

## How to start the queue

1. Create a `.env` file that contains `JWT_PASSWORD` and `TA_TOKEN`.
2. `JWT_PASSWORD` should be a fully random string of significant length.
3. `TA_TOKEN` is the login credential for teaching assistants and should be a secure password.
4. Build the project with `docker compose up --build`.

## How to use the queue

**For students**
1. Go to `http://localhost:27889/ta`, replacing `localhost` with the address of the server running the queue.
2. Click on the table where you are currently sitting.
3. Write a message for the TA in the text box.
4. Press the **Call a TA** button.
5. Wait for help, or press the **Leave Queue** button.

**For TAs**
1. Go to `http://localhost:27889/ta`.
2. Enter the `TA_TOKEN` configured in the `.env` file and press Enter. You will automatically be redirected to `http://localhost:27889/admin`.
3. Write your name in the text box.
4. You can now manage the queue. Press **Help** to notify the students that you are on the way, or remove students from the queue.

## I want to use this for a different room

**This feature has not been properly tested.**

1. Create a room layout in `HTMLServer/HTMLlogic/templates/rooms`: both `ROOM_NAME.css` and `ROOM_NAME.html`.
2. Each table should have an integer `id` starting from `1` and increasing.
3. In `main.rs`, update `static TABLE_AMOUNT: u8 = 30;` to the correct number of tables.
4. In `.env`, set `ROOM_NAME` to the name of the room. It should match the name of the HTML/CSS files.

## Debug steps

**The WS server does not start**

Try modifying a debug string in `main.rs` and rerun `docker compose up --build`.

## Future improvements
- Finish refactoring
- Make it use HTTPS instead of HTTP
- Make it work for multiple rooms at the same time
