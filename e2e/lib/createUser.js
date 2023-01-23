import { MnemonicKey } from "jmes/build/Client/providers/LCDClient/key/index.js";

function createUser(mnemonic) {
  const mnemonicKeyOptions = { mnemonic };
  const key = new MnemonicKey(mnemonicKeyOptions);

  const address = key.accAddress;

  return { address, mnemonicKeyOptions };
}

export { createUser };
