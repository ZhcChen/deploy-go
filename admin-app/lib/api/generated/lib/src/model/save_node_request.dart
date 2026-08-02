//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'save_node_request.g.dart';

/// SaveNodeRequest
///
/// Properties:
/// * [host]
/// * [name]
/// * [port]
/// * [secretsRoot]
/// * [sshCredentialId]
/// * [username]
/// * [version]
/// * [workRoot]
@BuiltValue()
abstract class SaveNodeRequest implements Built<SaveNodeRequest, SaveNodeRequestBuilder> {
  @BuiltValueField(wireName: r'host')
  String get host;

  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'port')
  int get port;

  @BuiltValueField(wireName: r'secrets_root')
  String get secretsRoot;

  @BuiltValueField(wireName: r'ssh_credential_id')
  String? get sshCredentialId;

  @BuiltValueField(wireName: r'username')
  String get username;

  @BuiltValueField(wireName: r'version')
  int? get version;

  @BuiltValueField(wireName: r'work_root')
  String get workRoot;

  SaveNodeRequest._();

  factory SaveNodeRequest([void updates(SaveNodeRequestBuilder b)]) = _$SaveNodeRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SaveNodeRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SaveNodeRequest> get serializer => _$SaveNodeRequestSerializer();
}

class _$SaveNodeRequestSerializer implements PrimitiveSerializer<SaveNodeRequest> {
  @override
  final Iterable<Type> types = const [SaveNodeRequest, _$SaveNodeRequest];

  @override
  final String wireName = r'SaveNodeRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SaveNodeRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'host';
    yield serializers.serialize(
      object.host,
      specifiedType: const FullType(String),
    );
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    yield r'port';
    yield serializers.serialize(
      object.port,
      specifiedType: const FullType(int),
    );
    yield r'secrets_root';
    yield serializers.serialize(
      object.secretsRoot,
      specifiedType: const FullType(String),
    );
    if (object.sshCredentialId != null) {
      yield r'ssh_credential_id';
      yield serializers.serialize(
        object.sshCredentialId,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'username';
    yield serializers.serialize(
      object.username,
      specifiedType: const FullType(String),
    );
    if (object.version != null) {
      yield r'version';
      yield serializers.serialize(
        object.version,
        specifiedType: const FullType.nullable(int),
      );
    }
    yield r'work_root';
    yield serializers.serialize(
      object.workRoot,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SaveNodeRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SaveNodeRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'host':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.host = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'port':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.port = valueDes;
          break;
        case r'secrets_root':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.secretsRoot = valueDes;
          break;
        case r'ssh_credential_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.sshCredentialId = valueDes;
          break;
        case r'username':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.username = valueDes;
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.version = valueDes;
          break;
        case r'work_root':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.workRoot = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SaveNodeRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SaveNodeRequestBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
