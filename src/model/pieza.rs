use crate::model::casilla::Casilla;
use crate::model::color::Color;
use crate::model::info::Info;

///Representa una pieza del ajedrez. Contiene la información de la misma.
///Define comportamiento común a todas las piezas.
#[derive(Debug, PartialEq)]
pub enum Pieza {
    Dama(Info),
    Rey(Info),
    Torre(Info),
    Peon(Info),
    Alfil(Info),
    Caballo(Info),
}

impl Pieza {
    ///Funcion que devuelve la información de la pieza correspondiente.
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

    ///Funcion que devuelve true si la pieza puede capturar a otra adyacente a la misma. False en caso contrario.
    fn puede_capturar_adyacente(&self, casilla_1: &Casilla, casilla_2: &Casilla) -> bool {
        let x = (casilla_1.fila - casilla_2.fila).abs();
        let y = (casilla_1.columna - casilla_2.columna).abs();
        ((x * x + y * y) as f64).sqrt() as i32 == 1
    }

    ///Funcion que devuelve true si la pieza puede capturar a otra en dirección diagonal. False en caso contrario.
    fn puede_capturar_diagonal(&self, casilla_1: &Casilla, casilla_2: &Casilla) -> bool {
        (casilla_1.fila - casilla_2.fila).abs() == (casilla_1.columna - casilla_2.columna).abs()
    }

    ///Funcion que devuelve true si la pieza puede capturar a otra en dirección recta (misma fila o columna). False en caso contrario.
    fn puede_capturar_recta(&self, casilla_1: &Casilla, casilla_2: &Casilla) -> bool {
        casilla_1.fila == casilla_2.fila || casilla_1.columna == casilla_2.columna
    }

    ///Funcion que devuelve true si la pieza puede capturar a otra en dirección L. False en caso contrario.
    fn puede_capturar_l(&self, casilla_1: &Casilla, casilla_2: &Casilla) -> bool {
        let dif_fila = (casilla_1.fila - casilla_2.fila).abs();
        let dif_col = (casilla_1.columna - casilla_2.columna).abs();

        (dif_fila == 1 && dif_col == 2) || (dif_fila == 2 && dif_col == 1)
    }

    ///Funcion que devuelve true si un peón puede capturar a otra pieza, es decir, si esta se encuentra en la dirección del peón (si es blanco por encima de este, si es negro por debajo) en diagonal a una distancia de 1. False en caso contrario.   
    ///El parámetro color corresponde al de self.
    fn puede_capturar_peon(
        &self,
        casilla_1: &Casilla,
        casilla_2: &Casilla,
        color_peon: &Color,
    ) -> bool {
        if !self.puede_capturar_adyacente(casilla_1, casilla_2) {
            return false;
        }

        if *color_peon == Color::Blanco {
            casilla_1.fila < casilla_2.fila && self.puede_capturar_diagonal(casilla_1, casilla_2)
        } else {
            casilla_1.fila > casilla_2.fila && self.puede_capturar_diagonal(casilla_1, casilla_2)
        }
    }

    ///Función que define el comportamiento común a todas las piezas. Si puede capturar a la otra pieza devolverá true, sino false.
    pub fn puede_capturar(&self, otra: &Pieza) -> bool {
        match self {
            Pieza::Dama(info) => {
                self.puede_capturar_diagonal(&info.posicion, &otra.get_info().posicion)
                    || self.puede_capturar_recta(&info.posicion, &otra.get_info().posicion)
            }
            Pieza::Rey(info) => {
                self.puede_capturar_adyacente(&info.posicion, &otra.get_info().posicion)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distancia_manhattan() {
        let casilla_1 = Casilla {
            fila: 1,
            columna: 1,
        };
        let casilla_2 = Casilla {
            fila: 0,
            columna: 0,
        };
        let pieza = Pieza::Rey(Info {
            color: Color::Blanco,
            posicion: casilla_1,
        });
        assert!(pieza.puede_capturar_adyacente(
            &Casilla {
                fila: 1,
                columna: 1
            },
            &casilla_2
        ));
    }

    #[test]
    fn test_get_info() {
        let casilla = Casilla {
            fila: 1,
            columna: 1,
        };
        let info = Info {
            color: Color::Negro,
            posicion: casilla,
        };
        let pieza = Pieza::Dama(info);
        assert_eq!(
            pieza.get_info(),
            &Info {
                color: Color::Negro,
                posicion: Casilla {
                    fila: 1,
                    columna: 1
                }
            }
        );
    }

    #[test]
    fn test_puede_capturar_diagonal() {
        let casilla_1 = Casilla {
            fila: 1,
            columna: 1,
        };
        let casilla_2 = Casilla {
            fila: 3,
            columna: 3,
        };
        let pieza = Pieza::Alfil(Info {
            color: Color::Blanco,
            posicion: casilla_1,
        });
        assert!(pieza.puede_capturar_diagonal(
            &Casilla {
                fila: 1,
                columna: 1
            },
            &casilla_2
        ));
    }

    #[test]
    fn test_puede_capturar_recta() {
        let casilla_1 = Casilla {
            fila: 1,
            columna: 1,
        };
        let casilla_2 = Casilla {
            fila: 1,
            columna: 5,
        };
        let pieza = Pieza::Torre(Info {
            color: Color::Blanco,
            posicion: casilla_1,
        });
        assert!(pieza.puede_capturar_recta(
            &Casilla {
                fila: 1,
                columna: 1
            },
            &casilla_2
        ));
    }

    #[test]
    fn test_puede_capturar_l() {
        let casilla_1 = Casilla {
            fila: 1,
            columna: 1,
        };
        let casilla_2 = Casilla {
            fila: 2,
            columna: 3,
        };
        let pieza = Pieza::Caballo(Info {
            color: Color::Blanco,
            posicion: casilla_1,
        });
        assert!(pieza.puede_capturar_l(
            &Casilla {
                fila: 1,
                columna: 1
            },
            &casilla_2
        ));
    }

    #[test]
    fn test_puede_capturar_peon() {
        let casilla_1 = Casilla {
            fila: 2,
            columna: 1,
        };
        let casilla_2 = Casilla {
            fila: 3,
            columna: 2,
        };
        let pieza = Pieza::Peon(Info {
            color: Color::Blanco,
            posicion: casilla_1,
        });
        assert!(pieza.puede_capturar_peon(
            &Casilla {
                fila: 2,
                columna: 1
            },
            &casilla_2,
            &Color::Blanco
        ));
    }

    #[test]
    fn test_no_puede_capturar_peon() {
        let casilla_1 = Casilla {
            fila: 2,
            columna: 1,
        };
        let casilla_2 = Casilla {
            fila: 3,
            columna: 2,
        };
        let pieza = Pieza::Peon(Info {
            color: Color::Negro,
            posicion: casilla_1,
        });
        assert!(!pieza.puede_capturar_peon(
            &Casilla {
                fila: 2,
                columna: 1
            },
            &casilla_2,
            &Color::Negro
        ));
    }

    #[test]
    fn test_puede_capturar() {
        let casilla_1 = Casilla {
            fila: 1,
            columna: 1,
        };
        let casilla_2 = Casilla {
            fila: 3,
            columna: 3,
        };
        let pieza_1 = Pieza::Alfil(Info {
            color: Color::Blanco,
            posicion: casilla_1,
        });
        let pieza_2 = Pieza::Rey(Info {
            color: Color::Negro,
            posicion: casilla_2,
        });
        assert!(pieza_1.puede_capturar(&pieza_2));
    }
}
