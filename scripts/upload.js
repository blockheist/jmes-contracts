import dotenv from "dotenv";
dotenv.config({ path: `./.${process.env.NETWORK_ENV}.env` });

import { createClient } from "../e2e/lib/createClient.js";
import { createUser } from "./lib/createUser.js";
import { uploadContract } from "./lib/uploadContracts.js";

const contractToUpload = process.argv[2];

export const upload = async () => {
  const client = await createClient();

  const user = createUser(client, process.env.USER1_MNEMONIC);

  console.log(process.argv);
  await uploadContract(client, user, contractToUpload);
};

await upload();
