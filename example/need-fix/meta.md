in C:\Users\junbe\Projects\rustic-machine\src\data\

I want to make a struct (you can name it yourself) that recieve market data

it contains many std async udpsockets (epoll) and many async websockets (std-async-tungstenite) 

Mostly it only has only has either udpsockets (traditional) or websockets (crypto)

do not use tokio, use std-async

