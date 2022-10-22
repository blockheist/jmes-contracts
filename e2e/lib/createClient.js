import { LCDClient, MsgSend } from "@terra-money/terra.js";
import { MnemonicKey } from "@terra-money/terra.js";
import { MsgExecuteContract } from "@terra-money/terra.js";

async function createClient() {
  const LCDOptions = {
    URL: process.env.LCDURL,
    chainID: process.env.CHAINID,
  };
  console.log("LCDOptions :>> ", LCDOptions);
  const client = new LCDClient(LCDOptions);

  client.query = async function (contractAddr, query) {
    return await client.wasm.contractQuery(contractAddr, query);
  };

  client.execute = async function (user, contractAddr, executeMsg) {
    try {
      const msg = new MsgExecuteContract(
        user.address,
        contractAddr,
        executeMsg
      );

      const txOptions = {
        msgs: [msg],
      };

      const key = new MnemonicKey(user.mnemonicKeyOptions);

      const wallet = client.wallet(key);

      const tx = await wallet.createAndSignTx(txOptions);

      return await client.tx.broadcast(tx);
    } catch (err) {
      if (err.response && err.response.data) {
        console.error("ERROR:", err.response.data);
      } else {
        console.error("ERROR:", err);
      }

      throw err;
    }
  };

  client.send = async function (user, receiverAddr, coins) {
    try {
      const msg = new MsgSend(user.address, receiverAddr, coins);

      const txOptions = {
        msgs: [msg],
      };

      const key = new MnemonicKey(user.mnemonicKeyOptions);

      const wallet = client.wallet(key);

      const tx = await wallet.createAndSignTx(txOptions);

      return await client.tx.broadcast(tx);
    } catch (err) {
      if (err.response && err.response.data) {
        console.error("ERROR:", err.response.data);
      } else {
        console.error("ERROR:", err);
      }

      throw err;
    }
  };

  return client;
}

export { createClient };
