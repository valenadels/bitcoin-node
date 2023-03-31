use tp_individual::ajedrez::{inicializar_piezas, comenzar_juego, jugar_ajedrez, model::resultado::Resultado};


#[test]
fn test_ajedrez_n() {
    let piezas = inicializar_piezas(&String::from("N.txt")).unwrap();
    let tablero = comenzar_juego(&piezas).unwrap();
    assert_eq!(jugar_ajedrez(&tablero), Resultado::Negro);
}

#[test]
fn test_ajedrez_p() {
    let piezas = inicializar_piezas(&String::from("P.txt")).unwrap();
    let tablero = comenzar_juego(&piezas).unwrap();
    assert_eq!(jugar_ajedrez(&tablero), Resultado::Ninguno);
}


#[test]
fn test_ajedrez_b() {
    let piezas = inicializar_piezas(&String::from("B.txt")).unwrap();
    let tablero = comenzar_juego(&piezas).unwrap();
    assert_eq!(jugar_ajedrez(&tablero), Resultado::Blanco);
}


#[test]
fn test_ajedrez_e() {
    let piezas = inicializar_piezas(&String::from("E.txt")).unwrap();
    let tablero = comenzar_juego(&piezas).unwrap();
    assert_eq!(jugar_ajedrez(&tablero), Resultado::Empate);
}


#[test]
fn test_ajedrez_dimension_invalida() {
    assert_eq!(inicializar_piezas(&String::from("dimension_invalida.txt")).unwrap_err(), "Error: [Dimensión del tablero errónea]");
}