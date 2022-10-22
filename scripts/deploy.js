import dotenv from "dotenv";
dotenv.config({ path: `./.${process.env.NETWORK_ENV}.env` });

import { createClient } from "../e2e/lib/createClient.js";
import { createUser } from "./lib/createUser.js";
import { uploadContracts } from "./lib/uploadContracts.js";
import { instantiateContracts } from "./lib/instantiateContracts.js";

export const deploy = async () => {
  const client = await createClient();

  const user2 = createUser(client, process.env.USER2_MNEMONIC);

  await uploadContracts(client, user2);
  await instantiateContracts(client, user2, { cache: true });
};

await deploy();
