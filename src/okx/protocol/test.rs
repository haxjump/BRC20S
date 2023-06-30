use crate::okx::datastore::brc30::Pid;
use crate::okx::protocol::brc20::Num;
pub(crate) use {
  super::*, crate::inscription_id::InscriptionId, crate::okx::datastore::ScriptKey,
  crate::okx::protocol::brc30::deserialize_brc30_operation,
  crate::okx::protocol::brc30::hash::caculate_tick_id,
  crate::okx::protocol::brc30::operation::BRC30Operation,
  crate::okx::protocol::brc30::BRC30ExecutionMessage, crate::okx::protocol::brc30::Deploy,
  crate::SatPoint, bitcoin::Address, shadow_rs::new, std::str::FromStr,
};

pub(crate) fn mock_create_brc30_message(
  from: ScriptKey,
  to: ScriptKey,
  op: BRC30Operation,
) -> BRC30ExecutionMessage {
  let inscription_id =
    InscriptionId::from_str("1111111111111111111111111111111111111111111111111111111111111111i1")
      .unwrap();
  let txid = inscription_id.txid.clone();
  let old_satpoint =
    SatPoint::from_str("1111111111111111111111111111111111111111111111111111111111111111:1:1")
      .unwrap();
  let new_satpoint =
    SatPoint::from_str("1111111111111111111111111111111111111111111111111111111111111111:2:1")
      .unwrap();
  let msg = BRC30ExecutionMessage::new(
    &txid,
    &inscription_id,
    0,
    &None,
    &old_satpoint,
    &new_satpoint,
    &Some(from.clone()),
    &from,
    &to,
    &op,
  );
  msg
}

pub(crate) fn mock_deploy_msg(
  pool_type: &str,
  poll_number: u8,
  stake: &str,
  earn: &str,
  earn_rate: &str,
  dmax: &str,
  supply: &str,
  dec: u8,
  only: bool,
  from: &str,
  to: &str,
) -> (Deploy, BRC30ExecutionMessage) {
  let only = if only { Some("1".to_string()) } else { None };

  let supply_128 = Num::from_str(supply).unwrap().checked_to_u128().unwrap();

  let from_script_key = ScriptKey::from_address(Address::from_str(from).unwrap());
  let to_script_key = ScriptKey::from_address(Address::from_str(to).unwrap());

  let tickid = caculate_tick_id(earn, supply_128, dec, &from_script_key, &to_script_key);
  let pid = tickid.hex().to_string() + &*poll_number.to_string();
  let msg = Deploy::new(
    pool_type.to_string(),
    pid,
    stake.to_string(),
    earn.to_string(),
    earn_rate.to_string(),
    dmax.to_string(),
    Some(supply.to_string()),
    only,
    Some(dec.to_string()),
  );

  let execute_msg = mock_create_brc30_message(
    from_script_key,
    to_script_key,
    BRC30Operation::Deploy(msg.clone()),
  );
  (msg, execute_msg)
}
