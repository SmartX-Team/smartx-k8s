from typing import final, override
from nats.aio.client import Client
from nats.js.client import JetStreamContext
from pydantic import BaseModel

from .base import BinaryMessenger, MessengerBaseSettings


class NatsMessenger[
    I: BaseModel,
    O: BaseModel,
](BinaryMessenger[I, O]):
    '''
    A NATS Jetstream messenger class.
    '''

    def __init__(
        self,
        settings: MessengerBaseSettings,
        input_type: type[I],
    ) -> None:
        super().__init__(settings, input_type)
        self._nc = Client()
        self._js: JetStreamContext | None = None
        self._consumer: JetStreamContext.PushSubscription | None = None
        self._group_id = settings.messenger_group_id

    @final
    async def _nats(self) -> JetStreamContext:
        '''
        Return the NATS Jetstream context.
        '''

        # Connect to servers
        if not self._nc.is_connected:
            await self._nc.connect(
                servers=self._bootstrapper_servers,
            )

        # Initialize Jetstream context
        if self._js is None:
            self._js = self._nc.jetstream()
            if self._topic_sink:
                await self._js.add_stream(
                    name=self._group_id,
                    subjects=self._topic_sink,
                )
        return self._js

    @final
    @override
    async def consume_bytes(self) -> bytes:
        if self._consumer is None:
            js = await self._nats()
            self._consumer = await js.subscribe(
                subject=self._topic_src[0],
                queue=self._group_id,
                durable=self._group_id,
            )

        msg = await self._consumer.next_msg(timeout=None)
        await msg.ack()
        return msg.data

    @final
    @override
    async def produce_bytes(self, data: bytes) -> None:
        for topic in self._topic_sink:
            js = await self._nats()
            await js.publish(
                subject=topic,
                payload=data,
            )

    @final
    @override
    async def flush(self) -> None:
        return await self._nc.flush()

    @final
    @override
    async def terminate(self) -> None:
        if self._consumer is not None:
            await self._consumer.unsubscribe()
        if self._nc.is_connected:
            await self._nc.close()
