"use client";

import { useRouter } from "next/navigation";
import { authClient } from "@/lib/auth-client";

export function Abmelden() {
  const router = useRouter();
  return (
    <button
      type="button"
      className="knopf klein"
      onClick={async () => {
        await authClient.signOut();
        router.replace("/login");
        router.refresh();
      }}
    >
      Abmelden
    </button>
  );
}
