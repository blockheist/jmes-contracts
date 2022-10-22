import { MnemonicKey } from "@terra-money/terra.js";

function createUser(client, mnemonic) {
  const key = new MnemonicKey({ mnemonic });

  const address = key.accAddress;

  const wallet = client.wallet(key);

  return { key, address, wallet, client };
}

export { createUser };
