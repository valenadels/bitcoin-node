use super::{pieza::Pieza, resultado::Resultado};

///Representa el tablero de ajedrez.
/// Contiene las piezas que se encuentran en el mismo.
/// En el modelo simplificado que estamos considerando hay sólo 2, una negra y una blanca.
/// Ambas deben tener un lifetime 'a que sea igual o mayor al del tablero, sino no tendría sentido un tablero vacío.
///Una observación: El tablero se considera siempre empezando con (0,0) en la esquina inferior izquierda (como si se lo estuviera viendo desde blancas).
#[derive(Debug)]
pub struct Tablero<'a> {
    pub pieza_blanca: &'a Pieza,
    pub pieza_negra: &'a Pieza,
}

impl Tablero<'_> {
    ///Determina el resultado del juego:
    /// Blanco: indica que solo la pieza blanca pueden capturar.
    /// Negro: indica que solo la pieza negra pueden capturar.
    /// Empate: indica que ambas piezas pueden capturar.
    /// Ninguna: indica que ninguna pieza puede capturar.
    pub fn calcular_resultado(&self) -> Resultado {
        let blanca_captura = self.pieza_blanca.puede_capturar(&self.pieza_negra);
        let negra_captura = self.pieza_negra.puede_capturar(&self.pieza_blanca);
        if blanca_captura && negra_captura {
            Resultado::Empate
        } else if blanca_captura {
            Resultado::Blanco
        } else if negra_captura {
            Resultado::Negro
        } else {
            Resultado::Ninguno
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ajedrez::model::{
        color::Color, info::Info, pieza::Pieza, resultado::Resultado, tablero::Tablero,
    };

    #[test]
    fn test_resultado_negro() {
        let pieza_blanca = Pieza::Torre(Info::new(Color::Blanco, 0, 0));
        let pieza_negra = Pieza::Rey(Info::new(Color::Negro, 1, 1));

        let tablero = Tablero {
            pieza_blanca: &pieza_blanca,
            pieza_negra: &pieza_negra,
        };

        assert_eq!(tablero.calcular_resultado(), Resultado::Negro);
    }

    #[test]
    fn test_resultado_blanco() {
        let pieza_blanca = Pieza::Dama(Info::new(Color::Blanco, 0, 1));
        let pieza_negra = Pieza::Peon(Info::new(Color::Negro, 2, 1));

        let tablero = Tablero {
            pieza_blanca: &pieza_blanca,
            pieza_negra: &pieza_negra,
        };

        assert_eq!(tablero.calcular_resultado(), Resultado::Blanco);
    }

    #[test]
    fn test_resultado_empate() {
        let pieza_blanca = Pieza::Torre(Info::new(Color::Blanco, 0, 0));
        let pieza_negra = Pieza::Dama(Info::new(Color::Negro, 0, 7));

        let tablero = Tablero {
            pieza_blanca: &pieza_blanca,
            pieza_negra: &pieza_negra,
        };

        assert_eq!(tablero.calcular_resultado(), Resultado::Empate);
    }

    #[test]
    fn test_resultado_ninguno_gana() {
        let pieza_blanca = Pieza::Caballo(Info::new(Color::Blanco, 2, 0));
        let pieza_negra = Pieza::Peon(Info::new(Color::Negro, 1, 1));

        let tablero = Tablero {
            pieza_blanca: &pieza_blanca,
            pieza_negra: &pieza_negra,
        };

        assert_eq!(tablero.calcular_resultado(), Resultado::Ninguno);
    }
}
