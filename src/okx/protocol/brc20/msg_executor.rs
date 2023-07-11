use super::{
  params::{BIGDECIMAL_TEN, MAXIMUM_SUPPLY, MAX_DECIMAL_WIDTH},
  *,
};

use crate::okx::datastore::brc20 as store_brc20;
use crate::okx::datastore::ord as store_ord;

use crate::{
  okx::{
    datastore::brc20::{
      BRC20Error, Balance, DeployEvent, Event, InscripbeTransferEvent, MintEvent, Receipt, Tick,
      TokenInfo, TransferEvent, TransferInfo, TransferableLog,
    },
    protocol::{
      brc20::{Message, Mint, Operation},
      utils, BlockContext,
    },
  },
  Result,
};
use anyhow::anyhow;
use bigdecimal::num_bigint::Sign;
use bitcoin::Network;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionMessage {
  pub(self) txid: Txid,
  pub(self) inscription_id: InscriptionId,
  pub(self) inscription_number: i64,
  pub(self) old_satpoint: SatPoint,
  pub(self) new_satpoint: SatPoint,
  pub(self) from: ScriptKey,
  pub(self) to: Option<ScriptKey>,
  pub(self) op: Operation,
}

impl ExecutionMessage {
  pub fn from_message<'a, O: store_ord::OrdDataStoreReadOnly>(
    ord_store: &'a O,
    msg: &Message,
    network: Network,
  ) -> Result<Self> {
    Ok(Self {
      txid: msg.txid,
      inscription_id: msg.inscription_id,
      inscription_number: utils::get_inscription_number_by_id(msg.inscription_id, ord_store)?,
      old_satpoint: msg.old_satpoint,
      new_satpoint: msg
        .new_satpoint
        .ok_or(anyhow!("new satpoint cannot be None"))?,
      from: utils::get_script_key_on_satpoint(msg.old_satpoint, ord_store, network)?,
      // Only transfer operations will encounter the situation where new_satpoint.outpoint.txid != msg.txid.
      // The engraving operation has been filtered out in the previous resolve_message step.
      to: if msg.new_satpoint.unwrap().outpoint.txid == msg.txid {
        Some(utils::get_script_key_on_satpoint(
          msg.new_satpoint.unwrap(),
          ord_store,
          network,
        )?)
      } else {
        None
      },
      op: msg.op.clone(),
    })
  }
}

pub fn execute<'a, O: store_ord::OrdDataStoreReadOnly, N: store_brc20::DataStoreReadWrite>(
  context: BlockContext,
  ord_store: &'a O,
  brc20_store: &'a N,
  msg: &ExecutionMessage,
) -> Result<Option<Receipt>> {
  log::debug!("BRC20 execute message: {:?}", msg);
  let event = match &msg.op {
    Operation::Deploy(deploy) => {
      process_deploy(context, ord_store, brc20_store, &msg, deploy.clone())
    }
    Operation::Mint(mint) => process_mint(context, ord_store, brc20_store, &msg, mint.clone()),
    Operation::InscribeTransfer(transfer) => {
      process_inscribe_transfer(context, ord_store, brc20_store, &msg, transfer.clone())
    }
    Operation::Transfer(_) => process_transfer(context, ord_store, brc20_store, &msg),
  };

  let receipt = Receipt {
    inscription_id: msg.inscription_id,
    inscription_number: msg.inscription_number,
    old_satpoint: msg.old_satpoint,
    new_satpoint: msg.new_satpoint,
    from: msg.from.clone(),
    // redirect receiver to sender if transfer to conibase.
    to: msg.to.clone().map_or(msg.from.clone(), |v| v),
    op: msg.op.op_type(),
    result: match event {
      Ok(event) => Ok(event),
      Err(Error::BRC20Error(e)) => Err(e),
      Err(e) => return Err(anyhow!("BRC20 execute exception: {e}")),
    },
  };

  log::debug!("BRC20 message receipt: {:?}", receipt);
  brc20_store
    .add_transaction_receipt(&msg.txid, &receipt)
    .map_err(|e| anyhow!("failed to add transaction receipt to state! error: {e}"))?;

  Ok(Some(receipt))
}

fn process_deploy<'a, O: store_ord::OrdDataStoreReadOnly, N: store_brc20::DataStoreReadWrite>(
  context: BlockContext,
  _ord_store: &'a O,
  brc20_store: &'a N,
  msg: &ExecutionMessage,
  deploy: Deploy,
) -> Result<Event, Error<N>> {
  // ignore inscribe inscription to coinbase.
  let to_script_key = msg.to.clone().ok_or(BRC20Error::InscribeToCoinbase)?;

  let tick = deploy.tick.parse::<Tick>()?;
  let lower_tick = tick.to_lowercase();

  if let Some(_) = brc20_store
    .get_token_info(&lower_tick)
    .map_err(|e| Error::LedgerError(e))?
  {
    return Err(Error::BRC20Error(BRC20Error::DuplicateTick(
      lower_tick.as_str().to_string(),
    )));
  }

  let dec = Num::from_str(&deploy.decimals.map_or(MAX_DECIMAL_WIDTH.to_string(), |v| v))?
    .checked_to_u8()?;
  if dec > MAX_DECIMAL_WIDTH {
    return Err(Error::BRC20Error(BRC20Error::DecimalsTooLarge(dec)));
  }
  let base = BIGDECIMAL_TEN.checked_powu(dec as u64)?;

  let supply = Num::from_str(&deploy.max_supply)?;

  if supply.sign() == Sign::NoSign
    || supply > MAXIMUM_SUPPLY.to_owned()
    || supply.scale() > dec as i64
  {
    return Err(Error::BRC20Error(BRC20Error::InvalidSupply(
      supply.to_string(),
    )));
  }

  let limit = Num::from_str(&deploy.mint_limit.map_or(deploy.max_supply, |v| v))?;

  if limit.sign() == Sign::NoSign || limit > MAXIMUM_SUPPLY.to_owned() || limit.scale() > dec as i64
  {
    return Err(Error::BRC20Error(BRC20Error::MintLimitOutOfRange(
      lower_tick.as_str().to_string(),
      limit.to_string(),
    )));
  }

  let supply = supply.checked_mul(&base)?.checked_to_u128()?;
  let limit = limit.checked_mul(&base)?.checked_to_u128()?;

  let new_info = TokenInfo {
    inscription_id: msg.inscription_id,
    inscription_number: msg.inscription_number,
    tick,
    decimal: dec,
    supply,
    limit_per_mint: limit,
    minted: 0u128,
    deploy_by: to_script_key,
    deployed_number: context.blockheight,
    latest_mint_number: context.blockheight,
    deployed_timestamp: context.blocktime,
  };
  brc20_store
    .insert_token_info(&lower_tick, &new_info)
    .map_err(|e| Error::LedgerError(e))?;

  Ok(Event::Deploy(DeployEvent {
    supply,
    limit_per_mint: limit,
    decimal: dec,
    tick: new_info.tick,
  }))
}

fn process_mint<'a, O: store_ord::OrdDataStoreReadOnly, N: store_brc20::DataStoreReadWrite>(
  context: BlockContext,
  _ord_store: &'a O,
  brc20_store: &'a N,
  msg: &ExecutionMessage,
  mint: Mint,
) -> Result<Event, Error<N>> {
  // ignore inscribe inscription to coinbase.
  let to_script_key = msg.to.clone().ok_or(BRC20Error::InscribeToCoinbase)?;

  let tick = mint.tick.parse::<Tick>()?;
  let lower_tick = tick.to_lowercase();

  let token_info = brc20_store
    .get_token_info(&lower_tick)
    .map_err(|e| Error::LedgerError(e))?
    .ok_or(BRC20Error::TickNotFound(lower_tick.as_str().to_string()))?;

  let base = BIGDECIMAL_TEN.checked_powu(token_info.decimal as u64)?;

  let mut amt = Num::from_str(&mint.amount)?;

  if amt.scale() > token_info.decimal as i64 {
    return Err(Error::BRC20Error(BRC20Error::AmountOverflow(
      amt.to_string(),
    )));
  }

  amt = amt.checked_mul(&base)?;
  if amt.sign() == Sign::NoSign {
    return Err(Error::BRC20Error(BRC20Error::InvalidZeroAmount));
  }
  if amt > Into::<Num>::into(token_info.limit_per_mint) {
    return Err(Error::BRC20Error(BRC20Error::AmountExceedLimit(
      amt.to_string(),
    )));
  }
  let minted = Into::<Num>::into(token_info.minted);
  let supply = Into::<Num>::into(token_info.supply);

  if minted >= supply {
    return Err(Error::BRC20Error(BRC20Error::TickMinted(
      token_info.tick.as_str().to_string(),
    )));
  }

  // cut off any excess.
  let mut out_msg = None;
  amt = if amt.checked_add(&minted)? > supply {
    let new = supply.checked_sub(&minted)?;
    out_msg = Some(format!(
      "amt has been cut off to fit the supply! origin: {}, now: {}",
      amt.to_string(),
      new.to_string()
    ));
    new
  } else {
    amt
  };

  // get or initialize user balance.
  let mut balance = brc20_store
    .get_balance(&to_script_key, &lower_tick)
    .map_err(|e| Error::LedgerError(e))?
    .map_or(Balance::new(), |v| v);

  // add amount to available balance.
  balance.overall_balance = Into::<Num>::into(balance.overall_balance)
    .checked_add(&amt)?
    .checked_to_u128()?;

  // store to database.
  brc20_store
    .update_token_balance(&to_script_key, &lower_tick, balance)
    .map_err(|e| Error::LedgerError(e))?;

  // update token minted.
  let minted = minted.checked_add(&amt)?.checked_to_u128()?;
  brc20_store
    .update_mint_token_info(&lower_tick, minted, context.blockheight)
    .map_err(|e| Error::LedgerError(e))?;

  Ok(Event::Mint(MintEvent {
    tick: token_info.tick,
    amount: amt.checked_to_u128()?,
    msg: out_msg,
  }))
}

fn process_inscribe_transfer<
  'a,
  O: store_ord::OrdDataStoreReadOnly,
  N: store_brc20::DataStoreReadWrite,
>(
  _context: BlockContext,
  _ord_store: &'a O,
  brc20_store: &'a N,
  msg: &ExecutionMessage,
  transfer: Transfer,
) -> Result<Event, Error<N>> {
  // ignore inscribe inscription to coinbase.
  let to_script_key = msg.to.clone().ok_or(BRC20Error::InscribeToCoinbase)?;

  let tick = transfer.tick.parse::<Tick>()?;
  let lower_tick = tick.to_lowercase();

  let token_info = brc20_store
    .get_token_info(&lower_tick)
    .map_err(|e| Error::LedgerError(e))?
    .ok_or(BRC20Error::TickNotFound(lower_tick.as_str().to_string()))?;

  let base = BIGDECIMAL_TEN.checked_powu(token_info.decimal as u64)?;

  let mut amt = Num::from_str(&transfer.amount)?;

  if amt.scale() > token_info.decimal as i64 {
    return Err(Error::BRC20Error(BRC20Error::AmountOverflow(
      amt.to_string(),
    )));
  }

  amt = amt.checked_mul(&base)?;
  if amt.sign() == Sign::NoSign || amt > Into::<Num>::into(token_info.supply) {
    return Err(Error::BRC20Error(BRC20Error::AmountOverflow(
      amt.to_string(),
    )));
  }

  let mut balance = brc20_store
    .get_balance(&to_script_key, &lower_tick)
    .map_err(|e| Error::LedgerError(e))?
    .map_or(Balance::new(), |v| v);

  let overall = Into::<Num>::into(balance.overall_balance);
  let transferable = Into::<Num>::into(balance.transferable_balance);
  let available = overall.checked_sub(&transferable)?;
  if available < amt {
    return Err(Error::BRC20Error(BRC20Error::InsufficientBalance(
      available.to_string(),
      amt.to_string(),
    )));
  }

  balance.transferable_balance = transferable.checked_add(&amt)?.checked_to_u128()?;

  let amt = amt.checked_to_u128()?;
  brc20_store
    .update_token_balance(&to_script_key, &lower_tick, balance)
    .map_err(|e| Error::LedgerError(e))?;

  let inscription = TransferableLog {
    inscription_id: msg.inscription_id,
    inscription_number: msg.inscription_number,
    amount: amt,
    tick: token_info.tick.clone(),
    owner: to_script_key,
  };
  brc20_store
    .insert_transferable(&inscription.owner, &lower_tick, inscription.clone())
    .map_err(|e| Error::LedgerError(e))?;

  brc20_store
    .insert_inscribe_transfer_inscription(
      msg.inscription_id,
      TransferInfo {
        tick: token_info.tick,
        amt,
      },
    )
    .map_err(|e| Error::LedgerError(e))?;

  Ok(Event::InscribeTransfer(InscripbeTransferEvent {
    tick: inscription.tick,
    amount: amt,
  }))
}

fn process_transfer<'a, O: store_ord::OrdDataStoreReadOnly, N: store_brc20::DataStoreReadWrite>(
  _context: BlockContext,
  _ord_store: &'a O,
  brc20_store: &'a N,
  msg: &ExecutionMessage,
) -> Result<Event, Error<N>> {
  let transferable = brc20_store
    .get_transferable_by_id(&msg.from, &msg.inscription_id)
    .map_err(|e| Error::LedgerError(e))?
    .ok_or(BRC20Error::TransferableNotFound(msg.inscription_id))?;

  let amt = Into::<Num>::into(transferable.amount);

  if transferable.owner != msg.from {
    return Err(Error::BRC20Error(BRC20Error::TransferableOwnerNotMatch(
      msg.inscription_id,
    )));
  }

  let lower_tick = transferable.tick.to_lowercase();

  let token_info = brc20_store
    .get_token_info(&lower_tick)
    .map_err(|e| Error::LedgerError(e))?
    .ok_or(BRC20Error::TickNotFound(lower_tick.as_str().to_string()))?;

  // update from key balance.
  let mut from_balance = brc20_store
    .get_balance(&msg.from, &lower_tick)
    .map_err(|e| Error::LedgerError(e))?
    .map_or(Balance::new(), |v| v);

  let from_overall = Into::<Num>::into(from_balance.overall_balance);
  let from_transferable = Into::<Num>::into(from_balance.transferable_balance);

  let from_overall = from_overall.checked_sub(&amt)?.checked_to_u128()?;
  let from_transferable = from_transferable.checked_sub(&amt)?.checked_to_u128()?;

  from_balance.overall_balance = from_overall;
  from_balance.transferable_balance = from_transferable;

  brc20_store
    .update_token_balance(&msg.from, &lower_tick, from_balance)
    .map_err(|e| Error::LedgerError(e))?;

  // redirect receiver to sender if transfer to conibase.
  let mut out_msg = None;

  let to_script_key = if let None = msg.to.clone() {
    out_msg = Some(format!(
      "redirect receiver to sender, reason: transfer inscription to coinbase"
    ));
    msg.from.clone()
  } else {
    msg.to.clone().unwrap()
  };

  // update to key balance.
  let mut to_balance = brc20_store
    .get_balance(&to_script_key, &lower_tick)
    .map_err(|e| Error::LedgerError(e))?
    .map_or(Balance::new(), |v| v);

  let to_overall = Into::<Num>::into(to_balance.overall_balance);
  to_balance.overall_balance = to_overall.checked_add(&amt)?.checked_to_u128()?;

  brc20_store
    .update_token_balance(&to_script_key, &lower_tick, to_balance)
    .map_err(|e| Error::LedgerError(e))?;

  brc20_store
    .remove_transferable(&msg.from, &lower_tick, msg.inscription_id)
    .map_err(|e| Error::LedgerError(e))?;

  brc20_store
    .remove_inscribe_transfer_inscription(msg.inscription_id)
    .map_err(|e| Error::LedgerError(e))?;

  Ok(Event::Transfer(TransferEvent {
    msg: out_msg,
    tick: token_info.tick,
    amount: amt.checked_to_u128()?,
  }))
}
