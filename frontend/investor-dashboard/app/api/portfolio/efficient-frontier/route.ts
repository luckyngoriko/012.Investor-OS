import { NextRequest } from "next/server";
import { proxyToBackendApi } from "@/app/api/_backend-proxy";

export async function GET(request: NextRequest) {
  return proxyToBackendApi(request, "portfolio/efficient-frontier");
}
