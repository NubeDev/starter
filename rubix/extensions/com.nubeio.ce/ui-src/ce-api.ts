// `ce-api.ts` — typed wrappers over this extension's tools.
//
// SCAFFOLD: each wrapper just names the tool / template and forwards
// params. No state, no invalidation, no error handling — fill that in
// alongside the real backend.

import { callTool, fetchTemplate } from "./api";
import {
  EXTENSION_ID,
  type DeviceInput,
  type DeviceRow,
  type WiresheetGraph,
  type WriteResult,
} from "./types";

const tool = (name: string): string => `${EXTENSION_ID}.${name}`;

// ----- Device catalog CRUD --------------------------------------------

export function listDevices(): Promise<ReadonlyArray<DeviceRow>> {
  return fetchTemplate<DeviceRow>(`${EXTENSION_ID}.devices_list`, {});
}

export function createDevice(input: DeviceInput): Promise<WriteResult> {
  return callTool<WriteResult>(tool("device_create"), input);
}

export function updateDevice(input: DeviceInput): Promise<WriteResult> {
  return callTool<WriteResult>(tool("device_update"), input);
}

export function deleteDevice(deviceId: string): Promise<WriteResult> {
  return callTool<WriteResult>(tool("device_delete"), { device_id: deviceId });
}

// ----- Control-engine REST proxy --------------------------------------

export function engineStatus(deviceId: string): Promise<unknown> {
  return callTool(tool("engine_status"), { device_id: deviceId });
}

export function getWiresheet(deviceId: string): Promise<WiresheetGraph> {
  return callTool<WiresheetGraph>(tool("engine_wiresheet_get"), {
    device_id: deviceId,
  });
}

export function putWiresheet(graph: WiresheetGraph): Promise<WriteResult> {
  return callTool<WriteResult>(tool("engine_wiresheet_put"), graph);
}
