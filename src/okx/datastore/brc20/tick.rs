use super::*;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt::Formatter, str::FromStr};

pub const TICK_BYTE_COUNT: usize = 4;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tick([u8; TICK_BYTE_COUNT]);

impl FromStr for Tick {
  type Err = BRC20Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let bytes = s.as_bytes();

    if s.to_lowercase().len() != TICK_BYTE_COUNT {
      log::warn!("tick {s} to lowercase len not equal to TICK_BYTE_COUNT");
      return Err(BRC20Error::InvalidTickLen(s.to_string()));
    }

    if bytes.len() != TICK_BYTE_COUNT {
      return Err(BRC20Error::InvalidTickLen(s.to_string()));
    }
    Ok(Self(bytes.try_into().unwrap()))
  }
}

impl From<LowerTick> for Tick {
  fn from(lower_tick: LowerTick) -> Self {
    Self(lower_tick.0 .0)
  }
}

impl Tick {
  pub fn as_str(&self) -> &str {
    // NOTE: Tick comes from &str by from_str,
    // so it could be calling unwrap when convert to str
    std::str::from_utf8(self.0.as_slice()).unwrap()
  }

  pub fn to_lowercase(&self) -> LowerTick {
    LowerTick(Self::from_str(self.as_str().to_lowercase().as_str()).unwrap())
  }

  pub fn hex(&self) -> String {
    hex::encode(&self.0)
  }
}

impl Serialize for Tick {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    self.as_str().serialize(serializer)
  }
}

impl<'de> Deserialize<'de> for Tick {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    Self::from_str(&String::deserialize(deserializer)?)
      .map_err(|e| de::Error::custom(format!("deserialize tick error: {}", e)))
  }
}

impl Display for Tick {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowerTick(Tick);

impl LowerTick {
  pub fn as_str(&self) -> &str {
    self.0.as_str()
  }

  pub fn hex(&self) -> String {
    hex::encode(&self.0 .0)
  }

  pub fn min_hex() -> String {
    hex::encode(&[0u8; TICK_BYTE_COUNT])
  }

  pub fn max_hex() -> String {
    hex::encode(&[0xffu8; TICK_BYTE_COUNT])
  }
}

impl Display for LowerTick {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

impl Serialize for LowerTick {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    self.as_str().serialize(serializer)
  }
}

impl<'de> Deserialize<'de> for LowerTick {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    Tick::from_str(&String::deserialize(deserializer)?)
      .map(|tick| tick.to_lowercase())
      .map_err(|e| de::Error::custom(format!("deserialize tick error: {}", e)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn test_tick_unicode_lowercase() {
    assert!(Tick::from_str("XAİ").is_err());
    assert!("XAİ".parse::<Tick>().is_err());
    assert!(Tick::from_str("X。").is_ok());
    assert!("X。".parse::<Tick>().is_ok());
    assert!(Tick::from_str("aBc1").is_ok());
    assert!("aBc1".parse::<Tick>().is_ok());
  }
  #[test]
  fn test_tick_compare_ignore_case() {
    assert_ne!(Tick::from_str("aBc1"), Tick::from_str("AbC1"));

    assert_ne!(Tick::from_str("aBc1"), Tick::from_str("aBc2"));

    assert_eq!(
      Tick::from_str("aBc1").unwrap().to_lowercase(),
      Tick::from_str("AbC1").unwrap().to_lowercase(),
    );
    assert_ne!(
      Tick::from_str("aBc1").unwrap().to_lowercase(),
      Tick::from_str("AbC2").unwrap().to_lowercase(),
    );
  }

  #[test]
  fn test_tick_serialize() {
    let obj = Tick::from_str("Ab1;").unwrap();
    assert_eq!(serde_json::to_string(&obj).unwrap(), r##""Ab1;""##);
    assert_eq!(
      serde_json::to_string(&obj.to_lowercase()).unwrap(),
      r##""ab1;""##
    );
  }

  #[test]
  fn test_tick_deserialize() {
    assert_eq!(
      serde_json::from_str::<Tick>(r##""Ab1;""##).unwrap(),
      Tick::from_str("Ab1;").unwrap()
    );
    assert_eq!(
      serde_json::from_str::<LowerTick>(r##""ab1;""##).unwrap(),
      Tick::from_str("ab1;").unwrap().to_lowercase()
    );
  }
}
