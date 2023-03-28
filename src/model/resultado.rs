use std::fmt::{Display, Formatter, Error};

pub enum Resultado {
    Blanco,
    Negro,
    Empate,
    Ninguno
}

impl Display for Resultado {
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