import dotenv from "dotenv";
dotenv.config({ path: `./.${process.env.NETWORK_ENV}.env` });

import { createClient } from "../e2e/lib/createClient.js";
import { createUser } from "./lib/createUser.js";
import { uploadContracts } from "./lib/uploadContracts.js";
import { instantiateContracts } from "./lib/instantiateContracts.js";

export const deploy = async () => {
  const client = await createClient();

  const user = createUser(client, process.env.USER1_MNEMONIC);

  await uploadContracts(client, user);
  await instantiateContracts(client, user, { cache: true });
};

await deploy();
