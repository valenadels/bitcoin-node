use std::fmt::{Display, Error, Formatter};

///Enum que representa los posibles resultados del juego.
#[derive(PartialEq, Debug)]
pub enum Resultado {
    Blanco,
    Negro,
    Empate,
    Ninguno,
}

impl Display for Resultado {
    ///Esta función define un formato de display a la hora de imprimirse por pantalla un elemento del enum de Resultado, asignándole a Blanco una B, a Negro N, empate E y ninguno gana P.
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let r = match self {
            Resultado::Blanco => 'B',
            Resultado::Negro => 'N',
            Resultado::Empate => 'E',
            Resultado::Ninguno => 'P',
        };
        write!(f, "{}", r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let b = Resultado::Blanco;
        let n = Resultado::Negro;
        let e = Resultado::Empate;
        let p = Resultado::Ninguno;

        assert_eq!(format!("{}", b), "B");
        assert_eq!(format!("{}", n), "N");
        assert_eq!(format!("{}", e), "E");
        assert_eq!(format!("{}", p), "P");
    }
}
