import { LCDClient } from "@terra-money/terra.js";

async function createLCDClient() {
  const LCDOptions = {
    URL: process.env.LCDURL,
    chainID: process.env.CHAINID,
  };
  console.log("LCDOptions :>> ", LCDOptions);
  const client = new LCDClient(LCDOptions);

  return client;
}

export { createLCDClient };
