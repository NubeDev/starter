export const EXTENSION_ID = "com.rubix.geo";

export interface Pin {
  readonly pin_id: string;
  readonly layer_id: string | null;
  readonly name: string | null;
  readonly description: string | null;
  readonly lng: number;
  readonly lat: number;
  readonly geometry_type: "Point" | "LineString" | "Polygon";
  readonly geometry: object | null;
  readonly icon: string | null;
  readonly color: string | null;
  readonly actions: PinAction[];
  readonly props: Record<string, unknown>;
  readonly created_at: string;
  readonly updated_at: string;
}

export interface PinAction {
  id: string;
  label: string;
  kind: "template" | "tool" | "url";
  target: string;
  params?: Record<string, unknown>;
  display?: "table" | "json" | "text";
}

export interface MapLayer {
  readonly layer_id: string;
  readonly name: string;
  readonly description: string | null;
  readonly style_url: string | null;
  readonly visible: boolean;
  readonly min_zoom: number | null;
  readonly max_zoom: number | null;
  readonly color: string | null;
  readonly cluster: boolean;
  readonly sort_order: number;
  readonly created_at: string;
}

export interface WarehouseQueryResponse<R = Record<string, unknown>> {
  template: string;
  rows: ReadonlyArray<R>;
  count: number;
}
