//! Entwicklungstest: Basis plus Abschluss (Paket, Sicherung, Gegenprobe).
#[path = "beispiel/abschluss.rs"]
mod abschluss;
#[path = "beispiel/basis.rs"]
mod basis;

#[tokio::test]
async fn beispiel_abschluss_mit_basis() {
    let dir = tempfile::tempdir().unwrap();
    let mut b = basis::basis(dir.path().join("profil")).await;
    let pdf = abschluss::bauvorlagenpaket(&mut b).await;
    assert!(pdf.starts_with(b"%PDF"));
    let archiv = dir.path().join("Beispiel.zip");
    abschluss::sicherung(&b, &archiv).await;
    abschluss::gegenprobe(&archiv, &b.p.name).await;
}
