import { NextRequest } from "next/server";
import { proxyAuthToBackend } from "@/app/api/auth/_proxy";

export async function POST(request: NextRequest) {
  return proxyAuthToBackend(request, "refresh");
}

