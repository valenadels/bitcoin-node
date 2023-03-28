use crate::model::resultado::Resultado;
use crate::model::pieza::Pieza;

pub struct Tablero{
    pub pieza_blanca: Pieza,
    pub pieza_negra: Pieza
}

impl Tablero {
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