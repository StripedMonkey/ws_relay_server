


### Getting into rooms
* Creating a room
  * Send `"NewRoom"` as the first message when connecting
  * The WebSocket will respond with the following:
    * `{"JoinRoom":"<ROOM-ID>"}`
* Joining an already existing room
  * Send `{"JoinRoom":"<ROOM-ID>"}` as the first message when connecting

After rooms have been joined and created any messages sent by any client will be seen by all other clients in the room