use crate::okx::datastore::brc20;
use crate::okx::datastore::brc20s::{PledgedTick, TickId};
use crate::okx::protocol::brc20s::params::{NATIVE_TOKEN, TICK_BYTE_COUNT, TICK_ID_STR_COUNT};
use crate::okx::protocol::brc20s::{BRC20SError, Num};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct PassiveUnStake {
  // 10 letter identifier of the pool id + "#" + 2 letter of pool number
  #[serde(rename = "stake")]
  pub stake: String,

  // Amount to withdraw: States the amount of the brc-20 to withdraw.
  #[serde(rename = "amt")]
  pub amount: String,
}

impl PassiveUnStake {
  pub fn get_stake_tick(&self) -> PledgedTick {
    let stake = self.stake.as_str();
    match stake {
      NATIVE_TOKEN => PledgedTick::Native,
      _ => match self.stake.len() {
        TICK_BYTE_COUNT => PledgedTick::BRC20Tick(brc20::Tick::from_str(stake).unwrap()),
        TICK_ID_STR_COUNT => PledgedTick::BRC20STick(TickId::from_str(stake).unwrap()),
        _ => PledgedTick::Unknown,
      },
    }
  }
  pub fn validate_basics(&self) -> Result<(), BRC20SError> {
    if self.get_stake_tick() == PledgedTick::Unknown {
      return Err(BRC20SError::UnknownStakeType);
    }

    if let Some(iserr) = Num::from_str(self.amount.as_str()).err() {
      return Err(BRC20SError::InvalidNum(
        self.amount.clone() + iserr.to_string().as_str(),
      ));
    }
    Ok(())
  }
}
