use super::{casilla::Casilla, color::Color};

///Contenedor de información de una pieza. Contiene el color y la posición de la misma.
#[derive(Debug, PartialEq)]
pub struct Info {
    pub color: Color,
    pub posicion: Casilla,
}

impl Info {
    ///Crea una nueva instancia de Info totalmente incializada.
    pub fn new(color: Color, fila: i32, columna: i32) -> Info {
        Info {
            color: color,
            posicion: Casilla {
                fila: fila,
                columna: columna,
            },
        }
    }
}
