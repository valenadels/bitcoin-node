mod ajedez;
use ajedrez::{comenzar_juego, inicializar_piezas, jugar_ajedrez};

#[test]
fn test_ajedrez_n() {
    let piezas = inicializar_piezas("N.txt").unwrap();
    let tablero = comenzar_juego(&piezas).unwrap();
    assert_eq!(jugar_ajedrez(&tablero), Resultado::Negro);
}

#[test]
fn test_ajedrez_p() {
    let piezas = inicializar_piezas("P.txt").unwrap();
    let tablero = comenzar_juego(&piezas).unwrap();
    assert_eq!(jugar_ajedrez(&tablero), Resultado::Ninguno);
}


#[test]
fn test_ajedrez_b() {
    let piezas = inicializar_piezas("B.txt").unwrap();
    let tablero = comenzar_juego(&piezas).unwrap();
    assert_eq!(jugar_ajedrez(&tablero), Resultado::Blanco);
}


#[test]
fn test_ajedrez_e() {
    let piezas = inicializar_piezas("E.txt").unwrap();
    let tablero = comenzar_juego(&piezas).unwrap();
    assert_eq!(jugar_ajedrez(&tablero), Resultado::Empate);
}


#[test]
fn test_ajedrez_dimension_invalida() {
    let piezas = inicializar_piezas("dimension_invalida.txt").unwrap_error();
    assert_eq!(piezas, "Error: [Dimensión del tablero errónea]");
}