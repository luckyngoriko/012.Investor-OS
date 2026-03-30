import { NextRequest } from "next/server";
import { proxyToBackendApi } from "@/app/api/_backend-proxy";

export async function GET(request: NextRequest) {
  const { searchParams } = new URL(request.url);
  const params = searchParams.toString();
  const endpoint = params
    ? `predictions/history?${params}`
    : "predictions/history";
  return proxyToBackendApi(request, endpoint);
}
