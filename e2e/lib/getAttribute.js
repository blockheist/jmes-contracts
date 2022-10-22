function getAttribute(result, event, attribute) {
  try {
    return result.logs[0].events
      .find((e) => e.type === event)
      .attributes.find((e) => e.key === attribute).value;
  } catch (e) {
    console.log("result :>> ", result);
    console.error("error", e);
    process.exit(1);
  }
}

export { getAttribute };
