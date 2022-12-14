import { MnemonicKey } from "jmes/build/Client/providers/LCDClient/key/index.js";

function createUser(client, mnemonic) {
  const key = new MnemonicKey({ mnemonic });

  const address = key.accAddress;

  console.log("address :>> ", address);

  const wallet = client.wallet(key);

  return { key, address, wallet, client };
}

export { createUser };
