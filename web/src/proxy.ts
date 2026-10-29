import { NextResponse, type NextRequest } from "next/server";
import { aktuelleSitzung } from "@/lib/auth";

// Ohne gültige Sitzung ist nur die Anmeldeseite erreichbar. Die Sitzung wird
// vollständig gegen die Datenbank geprüft, nicht nur das Cookie.
export async function proxy(request: NextRequest) {
  const sitzung = await aktuelleSitzung(request.headers);
  if (sitzung) return NextResponse.next();

  const { pathname, search } = request.nextUrl;
  const ziel = new URL("/login", request.url);
  if (pathname !== "/") ziel.searchParams.set("weiter", pathname + search);
  return NextResponse.redirect(ziel);
}

export const config = {
  matcher: ["/((?!login$|api/auth/|_next/static/|_next/image|favicon.ico$|icon.svg$|robots.txt$).*)"],
};
