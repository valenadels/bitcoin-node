use std::fs::File;
use std::env;
use std::io::BufReader;
use std::{io, process};
mod model;
use crate::model::color::Color;
use crate::model::info::Info;
use crate::model::pieza::Pieza;
use crate::model::tablero::Tablero;

//Maximo numero de filas y columnas del tablero
const MAX_TABLERO: i32 = 8;

///Dado un caracter del archivo de texto, agrega la pieza correspondiente si es que encuentra coincidencia. 
/// Si es blanca (minuscula) la agrega a la posicion 0 de la tupla. Si es negra (mayuscula) la agrega a la posicion 1 de la tupla.
/// Si no encuentra coincidencia ni con '_', simplemente no agrega nada. 
fn match_pieza(caracter: char, piezas: &mut (Option<Pieza>, Option<Pieza>), fila: i32, columna: i32) {
    match caracter {
        'r' => piezas.0 = Some(Pieza::Rey(Info::new(Color::Blanco, fila, columna))),
        'd' => piezas.0 = Some(Pieza::Dama(Info::new(Color::Blanco, fila, columna))),
        't' => piezas.0 = Some(Pieza::Torre(Info::new(Color::Blanco, fila, columna))),
        'p' => piezas.0 = Some(Pieza::Peon(Info::new(Color::Blanco, fila, columna))),
        'a' => piezas.0 = Some(Pieza::Alfil(Info::new(Color::Blanco, fila, columna))),
        'c' => piezas.0 = Some(Pieza::Caballo(Info::new(Color::Blanco, fila, columna))),
        'D' => piezas.1 = Some(Pieza::Dama(Info::new(Color::Negro, fila, columna))),
        'R' => piezas.1 = Some(Pieza::Rey(Info::new(Color::Negro, fila, columna))),
        'T' => piezas.1 = Some(Pieza::Torre(Info::new(Color::Negro, fila, columna))),
        'P' => piezas.1 = Some(Pieza::Peon(Info::new(Color::Negro, fila, columna))),
        'A' => piezas.1 = Some(Pieza::Alfil(Info::new(Color::Negro, fila, columna))),
        'C' => piezas.1 = Some(Pieza::Caballo(Info::new(Color::Negro, fila, columna))),
        '_' => {}
        _ => {
            println!("ERROR: [{}]\n", "No es un caracter válido");
            process::exit(1) //TODO: reemplazar por devolver el error en main
        }
    }
}

///Remueve los espacios que separan las columnas del tablero
fn eliminar_espacios(_linea: String) -> String {
    _linea
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
}

///Dada una posicion del string (columna) retorna el caracter en esa posición o error si no se encuentra
fn obtener_caracter(_linea: &String, columna: i32) -> char {
    let caracter = _linea.chars().nth(columna as usize).unwrap_or_else(|| {
        println!("ERROR: [{}]\n", "No se pudo leer un caracter del archivo");
        process::exit(1) //TODO: reemplazar por devolver el error en main
    });
    caracter
}

///Abre el archivo del path y devuelve las líneas del mismo (de tipo `Lines<BufReader<File>>`) o error si el path es inválido .
fn leer_lineas(path: &String) -> io::Lines<BufReader<File>> {
    let archivo = File::open(path).unwrap(); //TODO: sacar unwrap
    return io::BufRead::lines(io::BufReader::new(archivo));
}

///Dadas dos piezas de lifetime 'a, crea un tablero con esas piezas y lo devuelve.
fn crear_tablero<'a>(pieza_blanca: &'a Pieza, pieza_negra: &'a Pieza) -> Tablero<'a> {
    Tablero {
        pieza_blanca: pieza_blanca,
        pieza_negra: pieza_negra,
    }
}

///Recibe las líneas del archivo (de tipo `Lines<BufReader<File>>`) y devuelve las piezas que se encuentran en el mismo.
/// Casos de retorno:
/// - (None, None): No se encontraron piezas en el archivo
/// - (Some(pieza), None): Solo se encontró una blanca en el archivo
/// - (None, Some(pieza)): Solo se encontró una negra en el archivo
/// - (Some(pieza1), Some(pieza2)): Se encontraron ambas piezas en el archivo (blanca y negra)
///    De esta manera, se podrá luego validar si las fichas son válidas o no.
/// Verificacion de las dimensiones del tablero:
/// - Para las columnas se leerán como máximo 8, si hay más, se descartan
/// - De existir más de 8 filas, se devolverá un error
/// En caso de algun error devolverá el mismo y lo mostrará por pantalla.
fn obtener_piezas(lineas: io::Lines<BufReader<File>>) -> (Option<Pieza>, Option<Pieza>){
    let mut piezas: (Option<Pieza>, Option<Pieza>) = (None, None);
    let mut fila = 0;

    for linea in lineas {
        let mut _linea = linea.unwrap_or_else(|err| {
            println!(
                "ERROR: [{}. Error: {err}]\n",
                "No se pudo leer una linea del archivo"
            );
            process::exit(1) //TODO; reemplazar por devolver el error en main
        });

        _linea = eliminar_espacios(_linea);

        for columna in 0..MAX_TABLERO {
            let caracter = obtener_caracter(&_linea, columna);
            match_pieza(caracter, &mut piezas, fila, columna);
        }

        fila += 1; //TODO: aca verifico dimension
    }
    
    piezas
}

///Función principal del juego. Recibe el path del archivo y ejecuta el mismo. Imprime un error por pantalla en caso de que no se encuentren las piezas requeridas o hayan errores internos.
/// Como resultado será B,N,E o P dependiendo de si la pieza blanca gana, la negra, hay empate o no gana ninguno.
fn juego_de_ajedrez(path: &String) {
    let lineas = leer_lineas(path);
    let piezas = obtener_piezas(lineas);

    match piezas {
       (Some(p1), Some(p2)) => {
           let tablero = crear_tablero(&p1, &p2);
            println!("{}", tablero.calcular_resultado());
       }
       _ => println!("Error: [{}]", "No se encontraron las piezas requeridas")
    }
}


///Función principal del programa. Recibe los argumentos de la línea de comandos y ejecuta el juego de ajedrez.
/// Deberá ejecutarse de la siguiente manera: `cargo run -- <path>`. Se recomienda pasar el path absoluto del archivo para evitar inconvenientes, pero podría ser <archivo.txt> si se encuentra en el directorio padre a src.
/// En caso de no recibir el path, se mostrará un error por pantalla.
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        //TODO: error ;
    }

    juego_de_ajedrez(&args[1]);
}


#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn test_eliminar_espacios(){
        let linea = String::from("a b c d e f g h");
        let linea_sin_espacios = eliminar_espacios(linea);
        assert_eq!(linea_sin_espacios, "abcdefgh");
    }

    #[test]
    fn test_obtener_caracter(){
        let linea = String::from("abcdefgh");
        let caracter = obtener_caracter(&linea, 0);
        assert_eq!(caracter, 'a');
    }

    #[test]
    fn test_obtener_piezas(){
       
    }

    #[test]
    fn test_match_pieza() {
        let mut piezas: (Option<Pieza>, Option<Pieza>) = (None, None);
        match_pieza('r', &mut piezas, 1, 1);
        assert_eq!(piezas.0.as_ref().unwrap(), &Pieza::Rey(Info::new(Color::Blanco, 1, 1)));

        match_pieza('D', &mut piezas, 2, 2);
        assert_eq!(piezas.1.as_ref().unwrap(), &Pieza::Dama(Info::new(Color::Negro, 2, 2)));

        match_pieza('_', &mut piezas, 3, 3);
        assert_eq!(piezas, (Some(Pieza::Rey(Info::new(Color::Blanco, 1, 1))), Some(Pieza::Dama(Info::new(Color::Negro, 2, 2)))));

        //match_pieza('x', &mut piezas, 4, 4); //descomentar despues de poner que devuelve el error
        assert_eq!(piezas, (Some(Pieza::Rey(Info::new(Color::Blanco, 1, 1))), Some(Pieza::Dama(Info::new(Color::Negro, 2, 2)))));
    }

    #[test]
    fn test_crear_tablero() {
        let pieza_blanca = Pieza::Peon(Info::new(Color::Blanco, 2, 1));
        let pieza_negra = Pieza::Peon(Info::new(Color::Negro, 7, 1));
        let tablero = crear_tablero(&pieza_blanca, &pieza_negra);
        assert_eq!(*tablero.pieza_blanca, pieza_blanca);
        assert_eq!(*tablero.pieza_negra, pieza_negra);
    }


}
