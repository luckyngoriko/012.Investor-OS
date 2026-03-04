import { NextRequest } from "next/server";
import { proxyToBackendApi } from "@/app/api/_backend-proxy";

export async function GET(request: NextRequest) {
  return proxyToBackendApi(request, "killswitch");
}

export async function POST(request: NextRequest) {
  return proxyToBackendApi(request, "killswitch");
}
