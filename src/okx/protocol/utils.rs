use crate::{
  okx::datastore::{ord::OrdDataStoreReadOnly, ScriptKey},
  InscriptionId, Result, SatPoint,
};
use anyhow::anyhow;
use bitcoin::Network;

pub(super) fn get_script_key_on_satpoint<'a, O: OrdDataStoreReadOnly>(
  satpoint: SatPoint,
  ord_store: &'a O,
  network: Network,
) -> Result<ScriptKey> {
  Ok(ScriptKey::from_script(
    &ord_store
      .get_outpoint_to_txout(satpoint.outpoint)
      .map_err(|e| anyhow!("failed to get tx out from state! error: {e}",))?
      .ok_or(anyhow!(
        "failed to get tx out! error: outpoint {} not found",
        satpoint.outpoint
      ))?
      .script_pubkey,
    network,
  ))
}

pub(super) fn get_inscription_number_by_id<'a, O: OrdDataStoreReadOnly>(
  inscription_id: InscriptionId,
  ord_store: &'a O,
) -> Result<i64> {
  Ok(
    ord_store
      .get_number_by_inscription_id(inscription_id)
      .map_err(|e| anyhow!("failed to get inscription number from state! error: {e}"))?
      .ok_or(anyhow!(
        "failed to get inscription number! error: inscription id {} not found",
        inscription_id
      ))?,
  )
}
