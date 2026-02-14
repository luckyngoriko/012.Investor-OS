import { getRequestConfig } from "next-intl/server";
import { getUserLocale } from "./user-locale";

export default getRequestConfig(async () => {
  // Get locale from user preference or default
  const locale = await getUserLocale();

  return {
    locale,
    messages: (await import(`../messages/${locale}.json`)).default,
    timeZone: "Europe/Sofia",
    now: new Date(),
  };
});
