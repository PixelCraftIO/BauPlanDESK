import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: { default: "BauPlanIO", template: "%s · BauPlanIO" },
  description: "BauPlanIO – Bauantragsverfahren vom Flurstück bis zur Baugenehmigung.",
  robots: { index: false, follow: false },
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="de">
      <body>{children}</body>
    </html>
  );
}
