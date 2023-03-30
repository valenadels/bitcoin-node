use std::env;

mod ajedrez;
use ajedrez::{inicializar_piezas, comenzar_juego, jugar_ajedrez};

///Función principal del programa. Recibe el argumento de la línea de comandos y ejecuta el juego de ajedrez.
/// Deberá ejecutarse de la siguiente manera: `cargo run -- <path>`. Se recomienda pasar el path absoluto del archivo para evitar inconvenientes, pero podría ser <archivo.txt> si se encuentra en el directorio padre a src.
/// En caso de no recibir el path, se mostrará un error por pantalla.
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("ERROR: [No se recibió el path del archivo]");
    } else {
        match inicializar_piezas(&args[1]) {
            Ok(piezas) => match comenzar_juego(&piezas) {
                Ok(tablero) => jugar_ajedrez(&tablero),
                Err(err) => println!("{}", err),
            },
            Err(err) => println!("{}", err),
        }
    }
}
