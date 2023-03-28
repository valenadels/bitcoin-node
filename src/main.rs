use std::fs::File;
use std::io::BufReader;
use std::{io, process};
mod model;
use crate::model::info::Info;
use crate::model::pieza::Pieza;
use crate::model::color::Color;
use crate::model::tablero::Tablero;


const MAX_TABLERO: i32 = 8; 
//const MAX_PIEZAS: usize = 2;

fn leer_lineas(path: &String) -> io::Lines<BufReader<File>> {
    let archivo = File::open(path).unwrap(); //TODO: sacar unwrap
    return io::BufRead::lines(io::BufReader::new(archivo)); 
}

fn crear_tablero(pieza_blanca: Pieza, pieza_negra: Pieza) -> Tablero{
    Tablero{pieza_blanca: pieza_blanca, pieza_negra: pieza_negra}
}

fn main() {
    let args: Vec<String> = env::args().collect();
    use std::env;

    if args.len() != 2 {
        //TODO: error ;
    }

    let path = &args[1];
    
    let lineas = leer_lineas(path);
    let mut fila = 0;
    //valores x dafault-> ver mejor forma de h
    let mut pieza_blanca = Pieza::Peon(Info::new(Color::Blanco, &0, &0));
    let mut pieza_negra = Pieza::Peon(Info::new(Color::Negro, &0, &0));


    for linea in lineas {
        let _linea = linea.unwrap_or_else(|err| {
            println!("ERROR: [{}. Error: {err}]\n", "No se pudo leer una linea del archivo");
            process::exit(1) //TODO; reemplazar por devolver el error en main
        });

        let _linea = _linea.chars().filter(|c| !c.is_whitespace()).collect::<String>();


        for columna in 0..MAX_TABLERO {
            let caracter = _linea.chars().nth(columna as usize).unwrap_or_else(|| {
                println!("ERROR: [{}]\n", "No se pudo leer un caracter del archivo");
                process::exit(1) //TODO: reemplazar por devolver el error en main
            });
            match caracter{
                'd' => pieza_blanca = Pieza::Dama(Info::new(Color::Blanco, &fila, &columna)),
                'r' => pieza_blanca = Pieza::Rey(Info::new(Color::Blanco, &fila, &columna)),
                't' => pieza_blanca = Pieza::Torre(Info::new(Color::Blanco, &fila, &columna)),
                'p' => pieza_blanca = Pieza::Peon(Info::new(Color::Blanco, &fila, &columna)),
                'a' => pieza_blanca = Pieza::Alfil(Info::new(Color::Blanco, &fila, &columna)),
                'c' => pieza_blanca = Pieza::Caballo(Info::new(Color::Blanco, &fila, &columna)),
                'D' => pieza_negra = Pieza::Dama(Info::new(Color::Negro, &fila, &columna)),
                'R' => pieza_negra = Pieza::Rey(Info::new(Color::Negro, &fila, &columna)),
                'T' => pieza_negra = Pieza::Torre(Info::new(Color::Negro, &fila, &columna)),
                'P' => pieza_negra = Pieza::Peon(Info::new(Color::Negro, &fila, &columna)),
                'A' => pieza_negra = Pieza::Alfil(Info::new(Color::Negro, &fila, &columna)),
                'C' => pieza_negra = Pieza::Caballo(Info::new(Color::Negro, &fila, &columna)),
                    _ => {
                    println!("ERROR: [{}]\n", "No es un caracter válido");
                    process::exit(1) //TODO: reemplazar por devolver el error en main
                }
            }
            fila += 1;
        }
    }

    let tablero = crear_tablero(pieza_blanca, pieza_negra);
    let resultado = tablero.calcular_resultado();
    println!("{}", resultado);


    //cerrar archivo!!
}

