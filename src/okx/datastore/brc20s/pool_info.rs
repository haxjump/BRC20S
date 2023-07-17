use super::*;
use crate::okx::datastore::brc20s::PledgedTick;
use crate::okx::protocol::brc20s::{params::PID_BYTE_COUNT, BRC20SError};
use crate::InscriptionId;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct Pid([u8; PID_BYTE_COUNT]);

impl FromStr for Pid {
  type Err = BRC20SError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let temp = s.to_lowercase();
    let bytes = temp.as_bytes();

    if bytes.len() != PID_BYTE_COUNT {
      return Err(BRC20SError::InvalidTickLen(s.to_string()));
    }
    Ok(Self(bytes.try_into().unwrap()))
  }
}

impl Pid {
  pub fn as_str(&self) -> &str {
    // NOTE: Pid comes from &str by from_str,
    // so it could be calling unwrap when convert to str
    std::str::from_utf8(self.0.as_slice()).unwrap()
  }

  pub fn hex(&self) -> String {
    hex::encode(&self.0)
  }

  pub fn min_hex() -> String {
    Self([0u8; PID_BYTE_COUNT]).hex()
  }

  pub fn max_hex() -> String {
    Self([0xffu8; PID_BYTE_COUNT]).hex()
  }
}

impl Serialize for Pid {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    self.as_str().serialize(serializer)
  }
}

impl<'de> Deserialize<'de> for Pid {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    Self::from_str(&String::deserialize(deserializer)?)
      .map_err(|e| de::Error::custom(format!("deserialize tick error: {}", e)))
  }
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub enum PoolType {
  Pool,
  Fixed,
  Unknown,
}

impl PoolType {
  pub fn to_string(&self) -> String {
    match self {
      PoolType::Pool => String::from("pool"),
      PoolType::Fixed => String::from("fixed"),
      PoolType::Unknown => String::from("unknown"),
    }
  }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct PoolInfo {
  pub pid: Pid,
  pub ptype: PoolType,
  pub inscription_id: InscriptionId,
  pub stake: PledgedTick,
  pub erate: u128,
  pub minted: u128,
  pub staked: u128,
  pub dmax: u128,
  pub acc_reward_per_share: String,
  pub last_update_block: u64,
  pub only: bool,
  pub deploy_block: u64,
  pub deploy_block_time: u32,
}

impl PoolInfo {
  pub fn new(
    pid: &Pid,
    ptype: &PoolType,
    inscription_id: &InscriptionId,
    stake: &PledgedTick,
    erate: u128,
    minted: u128,
    staked: u128,
    dmax: u128,
    acc_reward_per_share: String,
    last_update_block: u64,
    only: bool,
    deploy_block: u64,
    deploy_block_time: u32,
  ) -> Self {
    Self {
      pid: pid.clone(),
      ptype: ptype.clone(),
      inscription_id: inscription_id.clone(),
      stake: stake.clone(),
      erate,
      minted,
      staked,
      dmax,
      acc_reward_per_share,
      last_update_block,
      only,
      deploy_block,
      deploy_block_time,
    }
  }
}

impl std::fmt::Display for PoolInfo {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "PoolInfo {{ pid: {}, ptype: {}, stake: {},erate: {},minted: {},staked: {}, \
      dmax: {}, acc_reward_per_share: {}, last_update_block:{}}}",
      self.pid.as_str(),
      self.ptype.to_string(),
      self.stake.to_string(),
      self.erate,
      self.minted,
      self.staked,
      self.dmax,
      self.acc_reward_per_share,
      self.last_update_block
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_pid_compare_ignore_case() {
    assert_eq!(
      Pid::from_str("A012345679#01"),
      Pid::from_str("a012345679#01")
    );
    assert_ne!(
      Pid::from_str("A012345679#01"),
      Pid::from_str("A012345679#02")
    );
    assert_ne!(
      Pid::from_str("A112345679#01"),
      Pid::from_str("A012345679#01")
    );
  }

  #[test]
  fn test_pid_length_case() {
    assert_eq!(
      Pid::from_str("a012345679#"),
      Err(BRC20SError::InvalidTickLen("a012345679#".to_string()))
    );
    assert_eq!(
      Pid::from_str(""),
      Err(BRC20SError::InvalidTickLen("".to_string()))
    );

    assert_eq!(
      Pid::from_str("12345"),
      Err(BRC20SError::InvalidTickLen("12345".to_string()))
    );

    assert_eq!(
      Pid::from_str("1234567"),
      Err(BRC20SError::InvalidTickLen("1234567".to_string()))
    );
  }

  #[test]
  fn test_pid_serialize() {
    let obj = Pid::from_str("a012345679#01").unwrap();
    assert_eq!(serde_json::to_string(&obj).unwrap(), r##""a012345679#01""##);
  }

  #[test]
  fn test_pid_deserialize() {
    assert_eq!(
      serde_json::from_str::<Pid>(r##""a012345679#01""##).unwrap(),
      Pid::from_str("a012345679#01").unwrap()
    );
  }
}
