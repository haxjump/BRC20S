use crate::okx::protocol::brc30::params::MAX_DECIMAL_WIDTH;
use crate::okx::protocol::brc30::BRC30Error;
use bigdecimal::num_bigint::{BigInt, Sign, ToBigInt};
use bigdecimal::{BigDecimal, One, Signed, ToPrimitive, Zero};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(PartialEq, PartialOrd, Debug, Clone)]
pub struct Num(BigDecimal);

impl Num {
  pub fn new(num: BigDecimal) -> Self {
    Self(num)
  }

  pub fn zero() -> Self {
    Self(BigDecimal::zero())
  }

  // TODO check overflow
  pub fn checked_add(&self, other: &Num) -> Result<Self, BRC30Error> {
    Ok(Self(self.0.clone() + &other.0))
  }

  pub fn checked_sub(&self, other: &Num) -> Result<Self, BRC30Error> {
    if self.0 < other.0 {
      return Err(BRC30Error::Overflow {
        op: String::from("checked_sub"),
        org: self.clone(),
        other: other.clone(),
      });
    }

    Ok(Self(self.0.clone() - &other.0))
  }

  // TODO check overflow
  pub fn checked_mul(&self, other: &Num) -> Result<Self, BRC30Error> {
    Ok(Self(self.0.clone() * &other.0))
  }

  pub fn checked_div(&self, other: &Num) -> Result<Self, BRC30Error> {
    if other.0.is_zero() {
      return Err(BRC30Error::DivedZero);
    }

    Ok(Self(self.0.clone() / other.0.clone()))
  }

  pub fn checked_powu(&self, exp: u64) -> Result<Self, BRC30Error> {
    match exp {
      0 => Ok(Self(BigDecimal::one())),
      1 => Ok(Self(self.0.clone())),
      exp => {
        let mut result = self.0.clone();
        for _ in 1..exp {
          result = result * &self.0;
        }

        Ok(Self(result))
      }
    }
  }

  pub fn checked_to_u8(&self) -> Result<u8, BRC30Error> {
    if !self.0.is_integer() {
      return Err(BRC30Error::InvalidInteger(self.clone()));
    }
    Ok(self.0.clone().to_u8().ok_or(BRC30Error::Overflow {
      op: String::from("to_u8"),
      org: self.clone(),
      other: Self(BigDecimal::from(u8::MAX)),
    })?)
  }

  pub fn sign(&self) -> Sign {
    self.0.sign()
  }

  pub fn scale(&self) -> i64 {
    let (_, scale) = self.0.as_bigint_and_exponent();
    scale
  }

  pub fn checked_to_u128(&self) -> Result<u128, BRC30Error> {
    if !self.0.is_integer() {
      return Err(BRC30Error::InvalidInteger(self.clone()));
    }
    Ok(
      self
        .0
        .to_bigint()
        .ok_or(BRC30Error::InternalError(format!(
          "convert {} to bigint failed",
          self.0
        )))?
        .to_u128()
        .ok_or(BRC30Error::Overflow {
          op: String::from("to_u128"),
          org: self.clone(),
          other: Self(BigDecimal::from(BigInt::from(u128::MAX))), // TODO: change overflow error to others
        })?,
    )
  }

  pub fn truncate_to_u128(&self) -> Result<u128, BRC30Error> {
    Ok(
      self
        .0
        .to_bigint()
        .ok_or(BRC30Error::InternalError(format!(
          "convert {} to bigint failed",
          self.0
        )))?
        .to_u128()
        .ok_or(BRC30Error::Overflow {
          op: String::from("to_u128"),
          org: self.clone(),
          other: Self(BigDecimal::from(BigInt::from(u128::MAX))), // TODO: change overflow error to others
        })?,
    )
  }

  pub fn max(a: &Num, b: &Num) -> Self {
    if a.gt(b) {
      Num::from(a.clone())
    } else {
      Num::from(b.clone())
    }
  }

  pub fn is_positive_integer(&self) -> bool {
    self.0.is_positive() && self.scale() == 0
  }

  pub fn is_less_than_max_u64(&self) -> bool {
    self.0 <= Into::<BigDecimal>::into(u64::MAX)
  }
}

impl From<u64> for Num {
  fn from(n: u64) -> Self {
    Self(BigDecimal::from(n))
  }
}

impl From<u128> for Num {
  fn from(n: u128) -> Self {
    Self(BigDecimal::from(BigInt::from(n)))
  }
}

impl FromStr for Num {
  type Err = BRC30Error;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    if s.starts_with(".") || s.ends_with(".") || s.find(&['e', 'E', '+', '-']).is_some() {
      return Err(BRC30Error::InvalidNum(s.to_string()));
    }
    let num = BigDecimal::from_str(s).map_err(|_| BRC30Error::InvalidNum(s.to_string()))?;

    let (_, scale) = num.as_bigint_and_exponent();
    if scale > MAX_DECIMAL_WIDTH as i64 {
      return Err(BRC30Error::InvalidNum(s.to_string()));
    }

    Ok(Self(num))
  }
}

impl Display for Num {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    self.0.fmt(f)
  }
}

impl Serialize for Num {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let s = self.to_string();
    serializer.serialize_str(&s)
  }
}

impl<'de> Deserialize<'de> for Num {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?;
    Ok(Self(
      BigDecimal::from_str(&s).map_err(serde::de::Error::custom)?,
    ))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use bigdecimal::FromPrimitive;
  #[test]
  fn test_num_from_str2() {
    assert_eq!(
      Num::from_str("001").unwrap(),
      Num(BigDecimal::new(BigInt::from(1), 0)),
    );
    assert_eq!(
      Num::from_str("00.1").unwrap(),
      Num(BigDecimal::new(BigInt::from(1), 1)),
    );
    assert_eq!(
      Num::from_str("0.0").unwrap(),
      Num(BigDecimal::new(BigInt::from(0), 0)),
    );
    assert_eq!(
      Num::from_str("0.100").unwrap(),
      Num(BigDecimal::new(BigInt::from(1), 1)),
    );
    assert_eq!(
      Num::from_str("0").unwrap(),
      Num(BigDecimal::new(BigInt::from(0), 0)),
    );
    assert_eq!(
      Num::from_str("00.00100").unwrap(),
      Num(BigDecimal::new(BigInt::from(1), 3)),
    );
  }

  #[test]
  fn test_num_from_str() {
    assert!(Num::from_str(".1").is_err());
    assert_eq!(
      Num(BigDecimal::new(BigInt::from(0), 0)),
      Num::from_str("0").unwrap()
    );
    assert_eq!(
      Num(BigDecimal::new(BigInt::from(1), 0)),
      Num::from_str("001").unwrap()
    );
    assert_eq!(
      Num(BigDecimal::new(BigInt::from(1), 1)),
      Num::from_str("00.1").unwrap()
    );

    assert_eq!(
      Num(BigDecimal::new(BigInt::from(0), 0)),
      Num::from_str("0.0").unwrap()
    );
    assert_eq!(
      Num(BigDecimal::new(BigInt::from(1), 1)),
      Num::from_str("0.100").unwrap()
    );
    assert_eq!(
      Num(BigDecimal::new(BigInt::from(1), 3)),
      Num::from_str("00.00100").unwrap()
    );
    assert_eq!(
      Num(BigDecimal::new(BigInt::from(11), 1)),
      Num::from_str("1.1").unwrap()
    );
    assert_eq!(
      Num(BigDecimal::new(BigInt::from(11), 1)),
      Num::from_str("1.1000").unwrap()
    );
    assert_eq!(
      Num(BigDecimal::new(BigInt::from(101), 2)),
      Num::from_str("1.01").unwrap()
    );

    // can not be negative
    assert!(Num::from_str("-1.1").is_err());

    // number of decimal fractional can not exceed 18
    assert_eq!(
      Num(BigDecimal::new(BigInt::from(1_0000000000_00000001u64), 18)),
      Num::from_str("1.000000000000000001").unwrap()
    );
    assert!(Num::from_str("1.0000000000000000001").is_err());
  }

  #[test]
  fn test_invalid_num() {
    assert!(Num::from_str("").is_err());
    assert!(Num::from_str(" ").is_err());
    assert!(Num::from_str(".").is_err());
    assert!(Num::from_str(" 123.456").is_err());
    assert!(Num::from_str(".456").is_err());
    assert!(Num::from_str(".456 ").is_err());
    assert!(Num::from_str(" .456 ").is_err());
    assert!(Num::from_str(" 456").is_err());
    assert!(Num::from_str("456 ").is_err());
    assert!(Num::from_str("45 6").is_err());
    assert!(Num::from_str("123. 456").is_err());
    assert!(Num::from_str("123.-456").is_err());
    assert!(Num::from_str("123.+456").is_err());
    assert!(Num::from_str("+123.456").is_err());
    assert!(Num::from_str("123.456.789").is_err());
    assert!(Num::from_str("123456789.").is_err());
    assert!(Num::from_str("123456789.12345678901234567891").is_err());
  }

  #[test]
  fn test_num_serialize() {
    let num = Num::from_str("1.01").unwrap();
    let json = serde_json::to_string(&num).unwrap();
    assert_eq!(json.as_str(), "\"1.01\"");
  }

  #[test]
  fn test_num_deserialize() {
    let num = serde_json::from_str::<Num>("\"1.11\"").unwrap();
    assert_eq!(Num::from_str("1.11").unwrap(), num);
  }

  #[test]
  fn test_num_checked_add() {
    assert_eq!(
      Num::from_str("2"),
      Num::from_str("1")
        .unwrap()
        .checked_add(&Num::from_str("1").unwrap())
    );
    assert_eq!(
      Num::from_str("2.1"),
      Num::from_str("1")
        .unwrap()
        .checked_add(&Num::from_str("1.1").unwrap())
    );
    assert_eq!(
      Num::from_str("2.1"),
      Num::from_str("1.1")
        .unwrap()
        .checked_add(&Num::from_str("1").unwrap())
    );
    assert_eq!(
      Num::from_str("2.222"),
      Num::from_str("1.101")
        .unwrap()
        .checked_add(&Num::from_str("1.121").unwrap())
    );
  }

  #[test]
  fn test_num_checked_sub() {
    assert_eq!(
      Num::from_str("2"),
      Num::from_str("3")
        .unwrap()
        .checked_sub(&Num::from_str("1").unwrap())
    );
    assert_eq!(
      Num::from_str("2.1"),
      Num::from_str("3")
        .unwrap()
        .checked_sub(&Num::from_str("0.9").unwrap())
    );
    assert_eq!(
      Num::from_str("2.1"),
      Num::from_str("3.1")
        .unwrap()
        .checked_sub(&Num::from_str("1").unwrap())
    );
    assert_eq!(
      Num::from_str("2.222"),
      Num::from_str("3.303")
        .unwrap()
        .checked_sub(&Num::from_str("1.081").unwrap())
    );
  }

  #[test]
  fn test_to_u8() {
    assert_eq!(Num::from_str("2").unwrap().checked_to_u8().unwrap(), 2);
    assert_eq!(Num::from_str("255").unwrap().checked_to_u8().unwrap(), 255);
    assert_eq!(
      Num::from_str("256").unwrap().checked_to_u8().unwrap_err(),
      BRC30Error::Overflow {
        op: String::from("to_u8"),
        org: Num::from_str("256").unwrap(),
        other: Num(BigDecimal::from_u8(u8::MAX).unwrap()),
      }
    );

    let n = Num::from_str("15.00").unwrap();
    assert_eq!(n.checked_to_u8().unwrap(), 15u8);
  }

  #[test]
  fn test_max_value() {
    // brc20 protocol stipulate that a max integer value is 64 bit, and decimal has 18 numbers at most.
    let max = format!("{}.999999999999999999", u64::MAX);

    BigDecimal::from_str(&max).unwrap();
  }

  #[test]
  fn test_checked_powu_floatpoint() {
    let n = Num::from_str("3.7").unwrap();
    assert_eq!(n.checked_powu(0).unwrap(), Num::from_str("1").unwrap());
    assert_eq!(n.checked_powu(1).unwrap(), n);
    assert_eq!(n.checked_powu(2).unwrap(), Num::from_str("13.69").unwrap());
    assert_eq!(n.checked_powu(3).unwrap(), Num::from_str("50.653").unwrap());
    assert_eq!(
      n.checked_powu(5).unwrap(),
      Num::from_str("693.43957").unwrap()
    );
    assert_eq!(
      n.checked_powu(18).unwrap(),
      Num::from_str("16890053810.563300749953435929").unwrap()
    );
  }

  #[test]
  fn test_checked_powu_integer() {
    let n = Num::from_str("10").unwrap();
    assert_eq!(n.checked_powu(0).unwrap(), Num::from_str("1").unwrap());
    assert_eq!(n.checked_powu(1).unwrap(), n);
    assert_eq!(n.checked_powu(2).unwrap(), Num::from_str("100").unwrap());
    assert_eq!(n.checked_powu(3).unwrap(), Num::from_str("1000").unwrap());
    assert_eq!(n.checked_powu(5).unwrap(), Num::from_str("100000").unwrap());
    assert_eq!(
      n.checked_powu(18).unwrap(),
      Num::from_str("1000000000000000000").unwrap()
    );
  }

  #[test]
  fn test_truncate_to_u128() {
    let n = Num::from_str(&format!("{}", u128::MAX)).unwrap();
    assert_eq!(n.truncate_to_u128().unwrap(), u128::MAX);

    let n = Num::from_str(&format!("0")).unwrap();
    assert_eq!(n.truncate_to_u128().unwrap(), 0);

    let n = Num::from_str(&format!("{}{}", u128::MAX, 1)).unwrap();
    assert_eq!(
      n.truncate_to_u128().unwrap_err(),
      BRC30Error::Overflow {
        op: String::from("to_u128"),
        org: n,
        other: Num::from(u128::MAX),
      }
    );

    let n = Num::from_str(&format!("{}.{}", u128::MAX - 1, "33333")).unwrap();
    assert_eq!(n.truncate_to_u128().unwrap(), u128::MAX - 1);

    let n = Num::from_str(&format!("{}.{}", 0, "33333")).unwrap();
    assert_eq!(n.truncate_to_u128().unwrap(), 0);
    let a = BigDecimal::from_str(&format!("0.333"))
      .unwrap()
      .to_bigint()
      .unwrap();

    let n = Num::from_str("3140000000000000000.1230").unwrap();
    assert_eq!(n.truncate_to_u128().unwrap(), 3140000000000000000u128);

    let n = Num::from_str(&format!("{}.{}", u128::MAX - 1, "33333")).unwrap();
    assert_eq!(
      Num::from_str("1e2").unwrap_err(),
      BRC30Error::InvalidNum("1e2".to_string())
    );
    assert_eq!(
      Num::from_str("0e2").unwrap_err(),
      BRC30Error::InvalidNum("0e2".to_string())
    );

    assert_eq!(
      Num::from_str("100E2").unwrap_err(),
      BRC30Error::InvalidNum("100E2".to_string())
    );

    return;
  }

  #[test]
  fn test_checked_to_u128() {
    let n = Num::from_str(&format!("{}", u128::MAX)).unwrap();
    assert_eq!(n.checked_to_u128().unwrap(), u128::MAX);

    let n = Num::from_str(&format!("0")).unwrap();
    assert_eq!(n.checked_to_u128().unwrap(), 0);

    let n = Num::from_str(&format!("{}{}", u128::MAX, 1)).unwrap();
    assert_eq!(
      n.checked_to_u128().unwrap_err(),
      BRC30Error::Overflow {
        op: String::from("to_u128"),
        org: n,
        other: Num::from(u128::MAX),
      }
    );

    let n = Num::from_str(&format!("{}.{}", u128::MAX - 1, "33333")).unwrap();
    assert_eq!(
      n.checked_to_u128().unwrap_err(),
      BRC30Error::InvalidInteger(n)
    );

    let n = Num::from_str(&format!("{}.{}", 0, "33333")).unwrap();
    assert_eq!(
      n.checked_to_u128().unwrap_err(),
      BRC30Error::InvalidInteger(n)
    );
    let a = BigDecimal::from_str(&format!("0.333"))
      .unwrap()
      .to_bigint()
      .unwrap();

    let n = Num::from_str("3140000000000000000.00").unwrap();
    assert_eq!(n.checked_to_u128().unwrap(), 3140000000000000000u128);

    let n = Num::from_str(&format!("{}.{}", u128::MAX - 1, "33333")).unwrap();
    assert_eq!(
      Num::from_str("1e2").unwrap_err(),
      BRC30Error::InvalidNum("1e2".to_string())
    );
    assert_eq!(
      Num::from_str("0e2").unwrap_err(),
      BRC30Error::InvalidNum("0e2".to_string())
    );

    assert_eq!(
      Num::from_str("100E2").unwrap_err(),
      BRC30Error::InvalidNum("100E2".to_string())
    );
  }
}