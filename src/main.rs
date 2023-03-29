use std::fs::File;
use std::env;
use std::io::BufReader;
use std::{io, process};
mod model;
use crate::model::color::Color;
use crate::model::info::Info;
use crate::model::pieza::Pieza;
use crate::model::tablero::Tablero;

const MAX_TABLERO: i32 = 8;

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

fn eliminar_espacios(_linea: String) -> String {
    _linea
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
}

fn obtener_caracter(_linea: &String, columna: i32) -> char {
    let caracter = _linea.chars().nth(columna as usize).unwrap_or_else(|| {
        println!("ERROR: [{}]\n", "No se pudo leer un caracter del archivo");
        process::exit(1) //TODO: reemplazar por devolver el error en main
    });
    caracter
}

fn leer_lineas(path: &String) -> io::Lines<BufReader<File>> {
    let archivo = File::open(path).unwrap(); //TODO: sacar unwrap
    return io::BufRead::lines(io::BufReader::new(archivo));
}

fn crear_tablero<'a>(pieza_blanca: &'a Pieza, pieza_negra: &'a Pieza) -> Tablero<'a> {
    Tablero {
        pieza_blanca: pieza_blanca,
        pieza_negra: pieza_negra,
    }
}

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

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        //TODO: error ;
    }

    juego_de_ajedrez(&args[1]);
}


