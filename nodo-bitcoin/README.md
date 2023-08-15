# 23C1-Inoxidables
## Instrucciones para correr el programa
- `cargo run` corre el nodo bitcoin en modo servidor junto con su interfaz gráfica
- `cargo test` corre todos los tests del proyecto

Para correr el nodo de manera client y server a la vez:
- Abrir 2 terminales
- En una ejecutar `cargo run -- <path al archivo nodo_client.conf>`
- En la otra ejecutar `cargo run` (por default usa el archivo nodo.conf)

Recomendamos redireccionar la salida estándar del nodo a un archivo para tener mejor legibilidad

El nodo cliente recién puede correrse una vez que el servidor haya finalizado la descarga de bloques, sino no responderá

## Account de prueba
- Bitcoin address: mr1J99hL9xgGu7T5XHR4Y85DwUkuwLMmMQ
- Private key: 921Hgc17AqM74Jq3tQ1diP2qzgh1Xq84xnksgWLfrQnPP1wyJ4G

Aclaración: el formato de la private key debe ser WIF en base58 de 51 caracteres (empieza con 9).