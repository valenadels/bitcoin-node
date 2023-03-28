use crate::model::casilla::Casilla;
use crate::model::color::Color;

pub struct Info {
    pub color: Color,
    pub posicion: Casilla
}

impl Info {
    pub fn new(color: Color, fila: &i32, columna: &i32) -> Info {
        Info {
            color: color,
            posicion: Casilla {
                fila: *fila,
                columna: *columna
            }
        }
    }
    
}