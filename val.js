const json = Object({
  body: Object({
    ___type: String("script"),
    list: Array([
      String('"{"type":"default-"'),
      String("(payload.params.number)"),
      String('"-and-"'),
      String("(payload.params.number)"),
      String('"","value":""'),
      String("(payload.params.number)"),
    ]),
    script: String(
      '"{"type":"default-"+(payload.params.number)+"-and-"+(payload.params.number)+"","value":""+(payload.params.number)'
    ),
  }),
  headers: Object({ "content-type": String("application/json") }),
  status_code: Number(201),
});