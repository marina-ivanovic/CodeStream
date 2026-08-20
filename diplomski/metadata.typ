#let format_strane = "iso-b5"         // могуће вредности: iso-b5, a4
#let naslov = "Имплементација дистрибуираног система за колаборацију у реалном времену заснованог на CRDT структурама"
#let autor = "Марина Ивановић"

// На енглеском
#let naslov_eng = "Implementation of a Distributed Real-Time Collaboration System Based on CRDT Structures"
#let autor_eng = "Marina Ivanović"

#let indeks = "SV6/2022"

// Име и презиме ментора
#let mentor = "Игор Дејановић"
// Звање: редовни професор, ванредни професор, доцент
#let mentor_zvanje = "редовни професор"

// Скинути коментаре са одговарајућих линија
#let studijski_program = "Софтверско инжењерство и информационе технологије"
//#let studijski_program = "Рачунарство и аутоматика"
#let stepen = "Основне академске студије"
//#let stepen = "Основне академске студије"

#let godina = [#datetime.today().year()]

#let kljucne_reci = "Дистрибуирани системи, колаборативно уређивање, CRDT, RGA, реално време, микросервиси, WebSocket"
#let apstrakt = [
     У раду је приказано пројектовање и имплементација дистрибуираног система CodeStream за колаборативно уређивање програмског кода у реалном времену. Конфликти услед истовремених измена решавају се CRDT структуром података заснованом на RGA алгоритму. Систем чини скуп независних Rust микросервиса који комуницирају синхроно (HTTP) и асинхроно (RabbitMQ), уз JWT аутентификацију, синхронизацију стања документа преко Redis-а и изоловано извршавање кода унутар Docker контејнера.
]

// На енглеском
#let kljucne_reci_eng = "Distributed systems, real-time collaboration, CRDT, RGA, microservices, WebSocket"
#let apstrakt_eng = [
     This thesis presents the design and implementation of CodeStream, a distributed system for real-time collaborative source code editing. Conflicts caused by simultaneous edits are resolved using a CRDT structure based on the RGA algorithm. The system consists of independent Rust microservices communicating synchronously (HTTP) and asynchronously (RabbitMQ), with JWT authentication, Redis-based document state synchronization, and isolated code execution inside Docker containers.
]

// TODO: Текст задатка добијате од ментора. Овде је дат предлог текста на
#let zadatak = [
     #lorem(100)
]

// TODO: Датум одбране и чланове комисије добијате од ментора
#let datum_odbrane = "01.01.2025"
#let komisija_predsednik = "Петар Петровић"
#let komisija_predsednik_zvanje = "ванредни професор"
#let komisija_clan = "Марко Марковић"
#let komisija_clan_zvanje = "доцент"

// На енглеском уписати чланове на латиници
#let komisija_predsednik_eng = "Petar Petrović"
#let komisija_clan_eng = "Marko Marković"
#let mentor_eng = "Igor Dejanović"


// Ово даље углавном не треба мењати.

#let zvanje_eng = (
     "редовни професор": "full professor",
     "ванредни професор": "assoc. professor",
     "доцент": "asist. professor",
)
#let komisija_predsednik_zvanje_eng = zvanje_eng.at(komisija_predsednik_zvanje)
#let komisija_clan_zvanje_eng = zvanje_eng.at(komisija_clan_zvanje)
#let mentor_zvanje_eng = zvanje_eng.at(mentor_zvanje)


#let vrsta_rada = if stepen == "Мастер академске студије" {
    "Дипломски - мастер рад"
} else {
    "Дипломски - бечелор рад"
}

#let oblast = "Електротехничко и рачунарско инжењерство"
#let oblast_eng = "Electrical and Computer Engineering"
#let disciplina = "Примењене рачунарске науке и информатика"
#let disciplina_eng = "Applied computer science and informatics"

#import "funkcije.typ": *
// Поглавља/страна/цитата/табела/слика/графика/прилога
#let fizicki_opis = physical()
