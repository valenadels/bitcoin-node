use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Lines};
pub mod model;
use model::color::Color;
use model::info::Info;
use model::pieza::Pieza;
use model::tablero::Tablero;

//Maximo numero de filas y columnas del tablero
const MAX_TABLERO: i32 = 8;

///Dado un caracter del archivo de texto, agrega la pieza correspondiente si es que encuentra coincidencia.
/// Si es blanca (minuscula) la agrega a la posicion 0 de la tupla. Si es negra (mayuscula) la agrega a la posicion 1 de la tupla.
/// Si no encuentra coincidencia ni con '_', no agrega nada y devuelve un error String.
fn match_pieza(
    caracter: char,
    piezas: &mut (Option<Pieza>, Option<Pieza>),
    fila: i32,
    columna: i32,
) -> Result<(), String> {
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
            return Err(String::from(
                "Error: [El tablero contiene un caracter inválido]",
            ));
        }
    }
    Ok(())
}

///Remueve los espacios que separan las columnas del tablero
fn eliminar_espacios(_linea: String) -> String {
    _linea
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
}

///Dada una posicion del string (columna) retorna el caracter en esa posición o error si no se encuentra
fn obtener_caracter(_linea: &str, columna: i32) -> Result<char, String> {
    match _linea.chars().nth(columna as usize) {
        Some(c) => Ok(c),
        None => Err(String::from("No se pudo leer un caracter del archivo")),
    }
}

///Abre el archivo del path y devuelve las líneas del mismo (de tipo `Lines<BufReader<File>>`) o error si el path es inválido .
fn leer_lineas(path: &String) -> Result<io::Lines<BufReader<File>>, String> {
    match File::open(path) {
        Ok(archivo) => Ok(io::BufReader::new(archivo).lines()),
        Err(error) => Err(format!("Error: [{}]:", error)),
    }
}

///Dadas dos piezas de lifetime 'a, crea un tablero con esas piezas y lo devuelve.
fn crear_tablero<'a>(pieza_blanca: &'a Pieza, pieza_negra: &'a Pieza) -> Tablero<'a> {
    Tablero {
        pieza_blanca,
        pieza_negra,
    }
}

///Recibe las líneas del archivo (de tipo `Lines<BufReader<File>>`) y devuelve las piezas que se encuentran en el mismo.
/// Casos de retorno:
/// - (None, None): No se encontraron piezas en el archivo
/// - (Some(pieza), None): Solo se encontró una blanca en el archivo
/// - (None, Some(pieza)): Solo se encontró una negra en el archivo
/// - (Some(pieza1), Some(pieza2)): Se encontraron ambas piezas en el archivo (blanca y negra)
///    De esta manera, se podrá luego validar si las fichas son válidas o no.
/// Verifica las dimensiones del tablero, debe ser de 8x8.
/// En caso de algun error devolverá el mismo en formato String.
fn obtener_piezas(
    lineas: Lines<BufReader<File>>,
) -> Result<(Option<Pieza>, Option<Pieza>), String> {
    let mut piezas: (Option<Pieza>, Option<Pieza>) = (None, None);
    let mut fila = 0;

    for linea in lineas {
        let _linea = match linea {
            Ok(l) => eliminar_espacios(l),
            Err(err) => return Err(format!("ERROR: [{}]\n", err)),
        };

        for columna in 0..MAX_TABLERO {
            let caracter = obtener_caracter(&_linea, columna)?;
            match_pieza(caracter, &mut piezas, fila, columna)?;
        }

        fila += 1;
        if fila > MAX_TABLERO || _linea.chars().count() as i32 > MAX_TABLERO {
            return Err(String::from("Error: [Dimensión del tablero errónea]"));
        }
    }

    Ok(piezas)
}

///Función inicializadora de las piezas. Recibe el path del archivo con el tablero. Imprime un error de tipo String en caso de que hayan errores, por ejemplo path inválido.
/// Devuelve una tupla con las piezas. Si todo salió bien, se encontrará la pieza blanca en la posición 0 y la negra en la posición 1.
pub fn inicializar_piezas(path: &String) -> Result<(Option<Pieza>, Option<Pieza>), String> {
    let lineas = leer_lineas(path)?;
    obtener_piezas(lineas)
}

///Dadas las piezas, las pone en el tablero. En caso de que no se encuentren las piezas requeridas, devuelve un error.
pub fn comenzar_juego(piezas: &'_ (Option<Pieza>, Option<Pieza>)) -> Result<Tablero<'_>, String> {
    match piezas {
        (Some(blanca), Some(negra)) => Ok(crear_tablero(blanca, negra)),
        _ => Err(String::from(
            "Error: [No se encontraron las piezas requeridas]",
        )),
    }
}

pub fn jugar_ajedrez(tablero: &Tablero) -> model::resultado::Resultado {
    tablero.calcular_resultado()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eliminar_espacios() {
        let linea = String::from("a b c d e f g h");
        let linea_sin_espacios = eliminar_espacios(linea);
        assert_eq!(linea_sin_espacios, "abcdefgh");
    }

    #[test]
    fn test_obtener_caracter() {
        let linea = String::from("abcdefgh");
        let caracter = obtener_caracter(&linea, 0).unwrap();
        assert_eq!(caracter, 'a');
    }

    #[test]
    fn test_obtener_piezas() {}

    #[test]
    fn test_match_pieza() {
        let mut piezas: (Option<Pieza>, Option<Pieza>) = (None, None);
        match_pieza('r', &mut piezas, 1, 1).unwrap();
        assert_eq!(
            piezas.0.as_ref().unwrap(),
            &Pieza::Rey(Info::new(Color::Blanco, 1, 1))
        );

        match_pieza('D', &mut piezas, 2, 2).unwrap();
        assert_eq!(
            piezas.1.as_ref().unwrap(),
            &Pieza::Dama(Info::new(Color::Negro, 2, 2))
        );

        match_pieza('_', &mut piezas, 3, 3).unwrap();
        assert_eq!(
            piezas,
            (
                Some(Pieza::Rey(Info::new(Color::Blanco, 1, 1))),
                Some(Pieza::Dama(Info::new(Color::Negro, 2, 2)))
            )
        );

        match_pieza('x', &mut piezas, 4, 4).unwrap_err();
        assert_eq!(
            piezas,
            (
                Some(Pieza::Rey(Info::new(Color::Blanco, 1, 1))),
                Some(Pieza::Dama(Info::new(Color::Negro, 2, 2)))
            )
        );
    }

    #[test]
    fn test_crear_tablero() {
        let pieza_blanca = Pieza::Peon(Info::new(Color::Blanco, 2, 1));
        let pieza_negra = Pieza::Peon(Info::new(Color::Negro, 7, 1));
        let tablero = crear_tablero(&pieza_blanca, &pieza_negra);
        assert_eq!(*tablero.pieza_blanca, pieza_blanca);
        assert_eq!(*tablero.pieza_negra, pieza_negra);
    }

    #[test]
    fn test_comenzar_juego() {
        let blanca = Pieza::Peon(Info::new(Color::Blanco, 0, 0));
        let negra = Pieza::Dama(Info::new(Color::Negro, 2, 2));

        // Probando con piezas válidas
        let piezas_validas: (Option<Pieza>, Option<Pieza>) = (Some(blanca), Some(negra));
        let resultado_valido = comenzar_juego(&piezas_validas);
        assert!(resultado_valido.is_ok());

        // Probando con piezas inexistentes
        let piezas_nulas: (Option<Pieza>, Option<Pieza>) = (None, None);
        let resultado_nulo = comenzar_juego(&piezas_nulas);
        assert!(resultado_nulo.is_err());
        assert_eq!(
            resultado_nulo.unwrap_err(),
            "Error: [No se encontraron las piezas requeridas]"
        );

        // Probando con pieza negra inexistente
        let blanca = Pieza::Dama(Info::new(Color::Negro, 2, 2));
        let pieza_negra_nula: (Option<Pieza>, Option<Pieza>) = (Some(blanca), None);
        let resultado_negra_nula = comenzar_juego(&pieza_negra_nula);
        assert!(resultado_negra_nula.is_err());
        assert_eq!(
            resultado_negra_nula.unwrap_err(),
            "Error: [No se encontraron las piezas requeridas]"
        );
    }
}
