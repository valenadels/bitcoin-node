use ajedrez::{comenzar_juego, inicializar_piezas, jugar_ajedrez};
use std::env;
use tp_individual::ajedrez;

///Función principal del programa. Recibe el argumento de la línea de comandos y ejecuta el juego de ajedrez.
/// Deberá ejecutarse de la siguiente manera: `cargo run -- <path>`. Se recomienda pasar el path absoluto del archivo para evitar inconvenientes, pero podría ser <archivo.txt> si se encuentra en el directorio padre a src.
/// En caso de no recibir el path, se mostrará un error por pantalla. También en caso de que falle algo internamente.
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("ERROR: [No se recibió el path del archivo]");
    } else if let Ok(piezas) = inicializar_piezas(&args[1]) {
        if let Ok(tablero) = comenzar_juego(&piezas) {
            println!("{}", jugar_ajedrez(&tablero));
        } else {
            println!("Error: [No se encontraron las piezas requeridas]");
        }
    } else {
        println!("Error: [No se encontraron las piezas requeridas]");
    }
}
