Send limited game state data, hide unneeded data and also intentionally obscured data such as item names
Use AI to answer questions against items
Do item generation ahead of time, store at least the next 3 to use at any time, started from server creation
Use AI to verify a question is valid and format it before adding it to the queue
Ability to guess an item and get score from it
Key player can kick players
Client doing requests to server needs to be async as this pauses the game
Convert the clientsided code to use reqwests not ureq
