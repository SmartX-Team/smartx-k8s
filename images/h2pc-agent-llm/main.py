#!/usr/bin/env python

from abc import ABCMeta, abstractmethod
import logging
from pathlib import Path
from typing import Iterable, Literal, final, override

from jinja2 import Template
from kafka import KafkaConsumer, KafkaProducer
from ollama import Client as OllamaClient, Message as OllamaMessage
from openai import Client as OpenAIClient
from openai.types import chat as openai_chat
from pydantic import BaseModel, SecretStr
from pydantic_settings import BaseSettings, SettingsConfigDict
import yaml

# Load environment variables from `.env` file
try:
    from dotenv import load_dotenv
    load_dotenv()
except ImportError:
    pass


class AgentBaseSettings(BaseSettings):
    '''
    A base agent settings model class.
    '''

    model_config = SettingsConfigDict(
        env_file='.env',
        env_file_encoding='utf-8',
        extra='ignore',
    )

    messenger_src_count_min: int = 1
    messenger_src_count_max: int | None = None
    messenger_sink_count_min: int = 1
    messenger_sink_count_max: int | None = None

    prompt_path: Path
    verbose: bool = False


class AgentBase[S: AgentBaseSettings, I, O](metaclass=ABCMeta):
    '''
    A virtual agent base class.
    '''

    def __init__(self) -> None:
        self._logger = logging.getLogger(__name__)
        logging.basicConfig(
            encoding='utf-8',
            level=logging.INFO,
        )
        self._logger.info(f'Welcome to the {self.name}!')
        self._settings = self._settings_type()()  # type: ignore

    @classmethod
    @abstractmethod
    def _settings_type(cls) -> type[S]:
        '''
        Return the settings class type.
        '''
        pass

    @property
    @abstractmethod
    def name(self) -> str:
        '''
        Return the agent name.
        '''
        pass

    @final
    @property
    def settings(self) -> S:
        '''
        Return the settings.
        '''
        return self._settings

    @final
    def exec(self) -> None:
        '''
        Consume the process and execute the agent task.
        '''
        self._logger.info('Started agent')
        try:
            while True:
                inputs: list[I] = []
                while len(inputs) < self.settings.messenger_src_count_min:
                    new_input = self._consume()
                    if new_input is None:
                        break
                    inputs.append(new_input)

                for _ in range(self.settings.messenger_sink_count_min):
                    new_output = self._process(inputs)
                    if new_output is None:
                        break
                    self._produce(new_output)
        except KeyboardInterrupt:
            self._logger.info('CTRL+C captured')
            return self.terminate()

    def _consume(self) -> I | None:
        '''
        Read a data.
        '''
        return None

    @abstractmethod
    def _process(self, data: list[I]) -> O | None:
        '''
        Process the data.
        '''
        pass

    def _produce(self, data: O) -> None:
        '''
        Write the data.
        '''
        pass

    def terminate(self) -> None:
        '''
        Terminate the agent.
        '''
        pass


class AgentSettings(AgentBaseSettings):
    kafka_bootstrap_servers: str
    kafka_group_id: str
    kafka_topic_src: str | None = None
    kafka_topic_sink: str | None = None

    ollama_base_url: str | None = None
    ollama_model_name: str | None = None

    openai_api_key: SecretStr | None = None
    openai_base_url: str = 'https://api.openai.com'
    openai_model_name: str | None = None

    def kafka_bootstrap_servers_list(self) -> list[str]:
        return self.kafka_bootstrap_servers.split(',')

    def kafka_topic_src_list(self) -> list[str]:
        return self.kafka_topic_src.split(',') \
            if self.kafka_topic_src \
            else []

    def kafka_topic_sink_list(self) -> list[str]:
        return self.kafka_topic_sink.split(',') \
            if self.kafka_topic_sink \
            else []


class Message(BaseModel):
    role: Literal['assistant', 'system', 'user']
    '''The role of the messages author.'''

    name: str | None = None
    '''The optional name for the participant.'''

    content: str
    '''The content of the message.'''

    def build_ollama(self) -> OllamaMessage:
        return OllamaMessage(
            role=self.role,
            content=self.content,
        )

    def build_openai(self) -> openai_chat.ChatCompletionMessageParam:
        match self.role:
            case 'assistant':
                return openai_chat.ChatCompletionAssistantMessageParam(
                    role='assistant',
                    name=self.name or '',
                    content=self.content,
                )
            case 'system':
                return openai_chat.ChatCompletionSystemMessageParam(
                    role='system',
                    name=self.name or '',
                    content=self.content,
                )
            case 'user':
                return openai_chat.ChatCompletionUserMessageParam(
                    role='user',
                    name=self.name or '',
                    content=self.content,
                )


class PromptTemplate(BaseModel):
    operator: Literal['Index', 'Message'] = 'Message'
    inputs: list[Message]


class Agent(AgentBase[AgentSettings, Message, Message]):
    def __init__(self) -> None:
        super().__init__()

        bootstrapper_servers = self.settings.kafka_bootstrap_servers_list()
        topic_src_list = self.settings.kafka_topic_src_list()
        topic_sink_list = self.settings.kafka_topic_sink_list()

        if topic_src_list:
            self._consumer = KafkaConsumer(
                allow_auto_create_topics=False,
                auto_offset_reset='latest',
                bootstrap_servers=bootstrapper_servers,
                group_id=self.settings.kafka_group_id,
                metrics_enabled=True,
            )
            self._consumer.subscribe(topic_src_list)
            assert self._consumer.bootstrap_connected()
        else:
            self._consumer = None

        if topic_sink_list:
            self._producer = KafkaProducer(
                allow_auto_create_topics=False,
                bootstrap_servers=bootstrapper_servers,
                compression_type=None,
                metrics_enabled=True,
            )
        else:
            self._producer = None

        if self.settings.ollama_model_name is not None:
            self._llm = OllamaClient(
                host=self.settings.ollama_base_url,
            )
        elif self.settings.openai_api_key is not None and \
                self.settings.openai_model_name is not None:
            self._llm = OpenAIClient(
                api_key=self.settings.openai_api_key.get_secret_value(),
                base_url=self.settings.openai_base_url,
            )
        else:
            self._llm = None

        self._prompt: Template | None
        if self._llm is not None:
            with open(
                file=self.settings.prompt_path,
                encoding='utf-8',
            ) as fp:
                self._prompt = Template(fp.read())
        else:
            self._prompt = None

        self._topic_sink = topic_sink_list

    @property
    @final
    @override
    def name(self) -> str:
        return 'H2PC LLM Agent'

    @classmethod
    @final
    @override
    def _settings_type(cls) -> type[AgentSettings]:
        return AgentSettings

    @final
    @override
    def _consume(self) -> Message | None:
        if self._consumer is not None:
            return Message.model_validate_json(next(self._consumer).value)
        else:
            return None

    @final
    def _build_prompt_ollama(
        self,
        messages: Iterable[Message],
    ) -> Iterable[OllamaMessage]:
        return (
            message.build_ollama()
            for message in messages
        )

    @final
    def _build_prompt_openai(
        self,
        messages: Iterable[Message],
    ) -> Iterable[openai_chat.ChatCompletionMessageParam]:
        return (
            message.build_openai()
            for message in messages
        )

    @final
    def _build_prompt_template(
        self,
        data: list[Message],
    ) -> PromptTemplate:
        if self._prompt is None:
            raise Exception('Empty prompt')

        return PromptTemplate.model_validate(yaml.safe_load(
            stream=self._prompt.render(
                inputs=data,
            )
        ))

    @final
    @override
    def _process(self, data: list[Message]) -> Message | None:
        template = self._build_prompt_template(data)

        # Generate content
        content: str | None
        if isinstance(self._llm, OllamaClient):
            completion = self._llm.chat(
                model=self.settings.ollama_model_name or '',  # type: ignore
                messages=list(self._build_prompt_ollama(template.inputs)),
            )
            content = completion.message.content
        elif isinstance(self._llm, OpenAIClient):
            completion = self._llm.chat.completions.create(
                model=self.settings.openai_model_name,  # type: ignore
                messages=self._build_prompt_openai(template.inputs),
            )
            content = completion.choices[0].message.content
        else:
            return super()._process(data)

        # Validate output content
        if content is None:
            return None
        if self.settings.verbose:
            logging.info(f'Generated: {content}')

        # Parse content
        match template.operator:
            case 'Index':
                index = int(content)
                if index < 1 or index >= len(data):
                    return None
                return data[index - 1]
            case 'Message':
                return Message(
                    role='assistant',
                    content=content,
                )

    @final
    @override
    def _produce(self, data: Message | None) -> None:
        if data is not None and self._producer is not None:
            value = data.model_dump_json().encode('utf-8')
            for topic in self._topic_sink:
                return self._producer.send(topic, value)

    @final
    @override
    def terminate(self) -> None:
        if self._consumer is not None:
            self._consumer.close()
        if self._producer is not None:
            self._producer.close()


if __name__ == '__main__':
    Agent().exec()
