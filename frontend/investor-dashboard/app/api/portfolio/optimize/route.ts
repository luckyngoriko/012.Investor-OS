import { NextRequest } from "next/server";
import { proxyToBackendApi } from "@/app/api/_backend-proxy";

export async function POST(request: NextRequest) {
  return proxyToBackendApi(request, "portfolio/optimize");
}
