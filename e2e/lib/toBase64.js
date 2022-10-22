function toBase64(obj) {
  return Buffer.from(JSON.stringify(obj)).toString("base64");
}
export { toBase64 };
