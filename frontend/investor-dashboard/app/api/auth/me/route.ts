import { NextRequest } from "next/server";
import { proxyAuthToBackend } from "@/app/api/auth/_proxy";

export async function GET(request: NextRequest) {
  return proxyAuthToBackend(request, "me");
}

