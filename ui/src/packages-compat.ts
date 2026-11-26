// Проверка дрейфа типов между bindings и структурными копиями в packages/*.
// Пакеты не импортируют bindings (D-17), поэтому их DTO объявлены заново; здесь
// взаимная присваиваемость. Если поле разошлось, `svelte-check` краснеет.
import type { WsEvent as BindingsWsEvent } from './bindings-phase6';
import type { WsEvent as PackageWsEvent } from '@trackly/transport/ws';

type Assert<T extends true> = T;
type Mutual<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;

export type WsEventCompat = Assert<Mutual<BindingsWsEvent, PackageWsEvent>>;
