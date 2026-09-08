import { redirect } from "next/navigation";

/** Default locale entry — send `/` to English docs home. */
export default function RootPage() {
  redirect("/en");
}
