//! WR-02 ревью фазы 41.7: остановка серверного режима не имеет права бросать
//! УЖЕ НАЧАТЫЙ HTTP-запрос.
//!
//! Предмет — контракт `start_server`, а не конкретный обработчик. Очередь записи
//! НЕ ОТМЕНЯЕМА: `WriterHandle::execute` отправляет замыкание в mpsc и ждёт
//! oneshot, а дроп ожидающей стороны замыкание не отменяет — оно исполняется на
//! writer-потоке и коммитит, после чего `let _ = reply_tx.send(...)` молча
//! съедает недоставленный ответ. Значит «бросить соединение посреди запроса»
//! равносильно: запись ЗАКОММИЧЕНА, клиент получил транспортную ошибку,
//! рассылки `EntitiesChanged`/`NumberSpaceChanged` (они живут ПОСЛЕ `.await?`)
//! не выполнились. Пользователь повторяет отправку — дубль акта.
//!
//! Тест держит ДВА инварианта одновременно, потому что починка одного ломает
//! другой:
//!
//! * **D1 — начатый запрос доезжает до ответа.** Это собственно WR-02.
//! * **D2 — новые запросы на том же соединении уже не принимаются.** Это
//!   свойство, ради которого дроп соединения вводился в GAP-1: после
//!   «Остановить сервер» LAN-браузер не должен продолжать дёргать API по
//!   открытому keep-alive соединению. Без D2 «починкой» WR-02 годился бы
//!   простой отказ от закрытия соединений.
//!
//! Обработчик здесь — заглушка со сном: он стоит на месте «замыкание записи уже
//! в очереди и вот-вот закоммитит». Брать настоящий сервис не нужно и вредно —
//! момент отмены стал бы невоспроизводимым. Гарантия «WS всё равно закрывается
//! мгновенно» закреплена ОТДЕЛЬНЫМ тестом `ws_close_on_server_stop`.
//!
//! Данные вымышленные.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::routing::get;
use axum::Router;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;

use trackly_app::server::{start_server, tls};

const BUDGET: Duration = Duration::from_secs(60);
/// Сколько обработчик «работает» после того, как сообщил о старте. Должно быть
/// заметно больше задержки доставки отмены и заметно меньше `DRAIN_GRACE` (3 с).
const HANDLER_WORK: Duration = Duration::from_millis(400);
/// Запас на чтение ответа: грейс 3 с плюс накладные.
const READ_WINDOW: Duration = Duration::from_secs(10);
const BODY: &str = "drained-ok";

// ---------------------------------------------------------------------------
// TLS-клиент
// ---------------------------------------------------------------------------
//
// Дублирует верификатор из `ws_close_on_server_stop.rs` осознанно: общее место
// для тестовых хелперов — `tests/entities_support/`, но этот модуль обслуживает
// реестровый гейт `entities_changed_gate` и держится неизменным. Тащить в него
// TLS-клиент ради второго теста — связать несвязанное.

/// Клиентский верификатор, принимающий любой сертификат.
///
/// Доверие к self-signed сертификату ОРТОГОНАЛЬНО предмету теста (жизненный
/// цикл запроса при остановке сервера). Подписи рукопожатия проверяются
/// по-настоящему — подменён только путь валидации цепочки.
#[derive(Debug)]
struct AcceptAnyServerCert(Arc<rustls::crypto::CryptoProvider>);

impl rustls::client::danger::ServerCertVerifier for AcceptAnyServerCert {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

fn client_tls_config() -> rustls::ClientConfig {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .expect("safe default protocol versions")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AcceptAnyServerCert(provider)))
        .with_no_client_auth();
    // Сервер прибивает ALPN к http/1.1 (`tls::pin_http1_alpn`).
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    config
}

type TlsConn = tokio_rustls::client::TlsStream<TcpStream>;

async fn connect_tls(addr: SocketAddr) -> TlsConn {
    let tcp = TcpStream::connect(addr).await.expect("tcp connect");
    let connector = tokio_rustls::TlsConnector::from(Arc::new(client_tls_config()));
    let server_name = rustls::pki_types::ServerName::try_from("127.0.0.1").expect("server name");
    connector
        .connect(server_name, tcp)
        .await
        .expect("tls handshake")
}

// ---------------------------------------------------------------------------
// Сырой HTTP/1.1 поверх TLS
// ---------------------------------------------------------------------------
//
// Клиент написан на байтах намеренно: hyper подключён без фичи `client`, а
// добавлять её ради теста — менять граф зависимостей продакшн-сборки. Запрос
// keep-alive (без `Connection: close`), иначе D2 выполнился бы тривиально.

async fn write_request(conn: &mut TlsConn, path: &str) {
    let req = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    conn.write_all(req.as_bytes()).await.expect("write request");
    conn.flush().await.expect("flush request");
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Прочитать ОДИН HTTP-ответ. `None` — поток закончился, не отдав ответа
/// (именно так выглядит брошенное соединение со стороны клиента).
///
/// Конец потока считается и по `n == 0`, и по ошибке чтения: брошенное
/// соединение даёт рваный разрыв без TLS `close_notify`, и rustls сообщает о нём
/// как `UnexpectedEof`, а аккуратно закрытое — как чистый EOF. Для предмета
/// теста это ОДНО И ТО ЖЕ наблюдение («ответа не будет»), и браузерный `onclose`
/// наступает в обоих случаях, поэтому различать их незачем — иначе падение
/// приходило бы паникой внутри хелпера вместо внятного сообщения D1/D2.
async fn read_response(conn: &mut TlsConn) -> Option<String> {
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 2048];

    // Заголовки.
    let head_end = loop {
        if let Some(p) = find(&buf, b"\r\n\r\n") {
            break p + 4;
        }
        match conn.read(&mut tmp).await {
            Ok(0) | Err(_) => return None,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
        }
    };

    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let len: usize = head
        .lines()
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse::<usize>().ok())
        .expect("ответ без Content-Length — тестовый обработчик его всегда ставит");

    // Тело. Обрыв посреди тела — тоже «ответа не будет»: клиент не может
    // считать такой ответ полученным.
    while buf.len() - head_end < len {
        match conn.read(&mut tmp).await {
            Ok(0) | Err(_) => return None,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
        }
    }

    Some(head + &String::from_utf8_lossy(&buf[head_end..head_end + len]))
}

// ---------------------------------------------------------------------------
// Окружение
// ---------------------------------------------------------------------------

struct Harness {
    addr: SocketAddr,
    /// Дочерний токен ЭТОГО инстанса сервера — ровно то, что дёргает
    /// «Остановить сервер» (`build_server_toggle_tauri` → `handle.cancel.cancel()`).
    server_token: CancellationToken,
    /// Сообщает, что обработчик ВОШЁЛ в работу: до этого сигнала отменять токен
    /// бессмысленно — запрос ещё не «в полёте» и тест ничего не проверял бы.
    started: tokio::sync::mpsc::UnboundedReceiver<()>,
}

async fn start_harness() -> Harness {
    let (started_tx, started) = tokio::sync::mpsc::unbounded_channel();

    let router = Router::new().route(
        "/slow",
        get(move || {
            let tx = started_tx.clone();
            async move {
                // «Замыкание записи ушло в очередь и вот-вот закоммитит».
                let _ = tx.send(());
                tokio::time::sleep(HANDLER_WORK).await;
                BODY
            }
        }),
    );

    let bundle = tls::generate_self_signed("127.0.0.1").expect("tls bundle");
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");

    let server_token = CancellationToken::new();
    let token_clone = server_token.clone();
    tokio::spawn(async move {
        start_server(router, listener, bundle.acceptor, token_clone)
            .await
            .expect("start_server");
    });

    Harness {
        addr,
        server_token,
        started,
    }
}

// ---------------------------------------------------------------------------
// D1 + D2
// ---------------------------------------------------------------------------

/// Остановка сервера посреди запроса обязана дать запросу доехать до ответа
/// (D1) и при этом перестать принимать новые запросы на том же соединении (D2).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stopping_server_lets_in_flight_request_finish_but_refuses_new_ones() {
    tokio::time::timeout(BUDGET, async {
        let mut h = start_harness().await;
        let mut conn = connect_tls(h.addr).await;

        // Контроль: сервер действительно обслуживает это соединение. Без этого
        // последующее «ответ пришёл» могло бы ничего не значить.
        write_request(&mut conn, "/slow").await;
        h.started.recv().await.expect("обработчик не стартовал");
        let warmup = tokio::time::timeout(READ_WINDOW, read_response(&mut conn))
            .await
            .expect("контрольный ответ не пришёл за окно чтения")
            .expect("соединение закрылось ДО остановки сервера");
        assert!(
            warmup.contains("200") && warmup.contains(BODY),
            "контрольный ответ не похож на успешный: {warmup}"
        );

        // Запрос «в полёте»: обработчик вошёл в работу и спит.
        write_request(&mut conn, "/slow").await;
        h.started
            .recv()
            .await
            .expect("обработчик не стартовал по второму запросу");

        // Ровно то, что делает кнопка «Остановить сервер».
        h.server_token.cancel();

        // D1 — начатый запрос доезжает до ответа.
        let in_flight = tokio::time::timeout(READ_WINDOW, read_response(&mut conn))
            .await
            .expect("чтение ответа не уложилось в окно");
        let in_flight = in_flight.expect(
            "соединение брошено посреди запроса: клиент получил конец потока вместо ответа. \
             Очередь записи НЕ отменяема — замыкание всё равно коммитит, поэтому так \
             выглядит «запись применена, клиент об этом не знает, рассылки после .await не \
             выполнились», а повтор отправки создаёт дубль (WR-02)",
        );
        assert!(
            in_flight.contains("200") && in_flight.contains(BODY),
            "начатый до остановки запрос обязан получить свой полный ответ, пришло: {in_flight}"
        );

        // D2 — новые запросы на этом соединении больше не обслуживаются.
        write_request(&mut conn, "/slow").await;
        let after_stop = tokio::time::timeout(READ_WINDOW, read_response(&mut conn))
            .await
            .expect("чтение после остановки не уложилось в окно");
        assert!(
            after_stop.is_none(),
            "после остановки сервера соединение обязано закрыться, а не обслуживать \
             новые запросы — иначе LAN-браузер продолжает дёргать API по открытому \
             keep-alive соединению (ради этого дроп и вводился в GAP-1). Пришло: {after_stop:?}"
        );
    })
    .await
    .expect("test exceeded 60s budget");
}
