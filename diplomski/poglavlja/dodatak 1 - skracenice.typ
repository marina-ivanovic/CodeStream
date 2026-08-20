// Овде пишете скраћенице које сте користили у тексту

#import "../funkcije.typ": dodatak

// Због аутоматског бројања сваки додатак мора имати лабелу са префиксом
// dodatak- као у наслову испод.
= Списак коришћених скраћеница <dodatak-1>

#dodatak(
    table(
        table.header[*Скраћеница*][*Опис*],

        [AMQP], [Advanced Message Queuing Protocol (протокол за размену порука
            коришћен од стране RabbitMQ-а)],
        [API], [Application Programming Interface (апликациони програмски интерфејс)],
        [CRDT], [Conflict-free Replicated Data Type (структура података без конфликата за реплицирано усклађивање)],
        [HS256], [HMAC with SHA-256 (симетрични алгоритам за криптографско потписивање података, попут JWT токена, помоћу једног дељеног тајног кључа).],
        [HTTP], [HyperText Transfer Protocol (протокол за пренос хипертекста)],
        [JSON], [JavaScript Object Notation (формат за размену података)],
        [JWT], [JSON Web Token (сигурносни токен заснован на JSON формату)],
        [ОТ], [Операциона трансформација (енг. _Operational Transformation_)],
        [PDF], [Portable Document Format (преносиви формат документа)],
        [RGA], [Replicated Growable Array (реплицирани растући низ -- CRDT структура коришћена за синхронизацију документа)],
        [SPA], [Single Page Application (апликација са једном страном)],
        [URL], [Uniform Resource Locator (јединствени идентификатор и локатор ресурса)],
        [UUID], [Universally Unique Identifier (универзално јединствени идентификатор)],
        [WS], [WebSocket (протокол који омогућава двосмерну комуникацију у реалном времену преко једне дуготрајне TCP конекције)],
    ),
)
