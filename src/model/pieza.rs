use crate::model::casilla::Casilla;
use crate::model::color::Color;
use crate::model::info::Info;

#[derive(Debug)]
pub enum Pieza {
    Dama(Info),
    Rey(Info),
    Torre(Info),
    Peon(Info),
    Alfil(Info),
    Caballo(Info),
}

impl Pieza {
    fn distancia_manhattan(&self, casilla_1: &Casilla, casilla_2: &Casilla) -> i32 {
        let x = (casilla_1.fila - casilla_2.fila).abs();
        let y = (casilla_1.columna - casilla_2.columna).abs();
        x + y
    }

    fn get_info(&self) -> &Info {
        match self {
            Pieza::Dama(info) => info,
            Pieza::Rey(info) => info,
            Pieza::Torre(info) => info,
            Pieza::Peon(info) => info,
            Pieza::Alfil(info) => info,
            Pieza::Caballo(info) => info,
        }
    }

    fn puede_capturar_diagonal(&self, casilla_1: &Casilla, casilla_2: &Casilla) -> bool {
        (casilla_1.fila - casilla_2.fila).abs() == (casilla_1.columna - casilla_2.columna).abs()
    }

    fn puede_capturar_recta(&self, casilla_1: &Casilla, casilla_2: &Casilla) -> bool {
        casilla_1.fila == casilla_2.fila || casilla_1.columna == casilla_2.columna
    }

    fn puede_capturar_l(&self, casilla_1: &Casilla, casilla_2: &Casilla) -> bool {
        let dif_fila = (casilla_1.fila - casilla_2.fila).abs();
        let dif_col = (casilla_1.columna - casilla_2.columna).abs();

        (dif_fila == 1 && dif_col == 2) || (dif_fila == 2 && dif_col == 1)
    }

    fn puede_capturar_peon(
        &self,
        casilla_1: &Casilla,
        casilla_2: &Casilla,
        color_peon: &Color,
    ) -> bool {
        if self.distancia_manhattan(casilla_1, casilla_2) != 1 {
            return false;
        }

        if *color_peon == Color::Blanco {
            casilla_1.fila < casilla_2.fila && self.puede_capturar_diagonal(casilla_1, casilla_2)
        } else {
            casilla_1.fila > casilla_2.fila && self.puede_capturar_diagonal(casilla_1, casilla_2)
        }
    }

    pub fn puede_capturar(&self, otra: &Pieza) -> bool {
        match self {
            Pieza::Dama(info) => {
                self.puede_capturar_diagonal(&info.posicion, &otra.get_info().posicion)
                    || self.puede_capturar_recta(&info.posicion, &otra.get_info().posicion)
            }
            Pieza::Rey(info) => {
                self.distancia_manhattan(&info.posicion, &otra.get_info().posicion) == 1
                //TODO ver si es <=
            }
            Pieza::Torre(info) => {
                self.puede_capturar_recta(&info.posicion, &otra.get_info().posicion)
            }
            Pieza::Peon(info) => {
                self.puede_capturar_peon(&info.posicion, &otra.get_info().posicion, &info.color)
            }
            Pieza::Alfil(info) => {
                self.puede_capturar_diagonal(&info.posicion, &otra.get_info().posicion)
            }
            Pieza::Caballo(info) => {
                self.puede_capturar_l(&info.posicion, &otra.get_info().posicion)
            }
        }
    }
}
