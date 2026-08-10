//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'node_response.g.dart';

/// NodeResponse
///
/// Properties:
/// * [checkedAt]
/// * [createdAt]
/// * [host]
/// * [id]
/// * [name]
/// * [port]
/// * [privilegedExecution]
/// * [secretsRoot]
/// * [sshCredentialId]
/// * [status]
/// * [trustedHostFingerprint]
/// * [updatedAt]
/// * [username]
/// * [version]
/// * [workRoot]
@BuiltValue()
abstract class NodeResponse implements Built<NodeResponse, NodeResponseBuilder> {
  @BuiltValueField(wireName: r'checked_at')
  String? get checkedAt;

  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'host')
  String? get host;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'port')
  int? get port;

  @BuiltValueField(wireName: r'privileged_execution')
  bool get privilegedExecution;

  @BuiltValueField(wireName: r'secrets_root')
  String? get secretsRoot;

  @BuiltValueField(wireName: r'ssh_credential_id')
  String? get sshCredentialId;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'trusted_host_fingerprint')
  String? get trustedHostFingerprint;

  @BuiltValueField(wireName: r'updated_at')
  String get updatedAt;

  @BuiltValueField(wireName: r'username')
  String? get username;

  @BuiltValueField(wireName: r'version')
  int get version;

  @BuiltValueField(wireName: r'work_root')
  String? get workRoot;

  NodeResponse._();

  factory NodeResponse([void updates(NodeResponseBuilder b)]) = _$NodeResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(NodeResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<NodeResponse> get serializer => _$NodeResponseSerializer();
}

class _$NodeResponseSerializer implements PrimitiveSerializer<NodeResponse> {
  @override
  final Iterable<Type> types = const [NodeResponse, _$NodeResponse];

  @override
  final String wireName = r'NodeResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    NodeResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.checkedAt != null) {
      yield r'checked_at';
      yield serializers.serialize(
        object.checkedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
    if (object.host != null) {
      yield r'host';
      yield serializers.serialize(
        object.host,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    if (object.port != null) {
      yield r'port';
      yield serializers.serialize(
        object.port,
        specifiedType: const FullType.nullable(int),
      );
    }
    yield r'privileged_execution';
    yield serializers.serialize(
      object.privilegedExecution,
      specifiedType: const FullType(bool),
    );
    if (object.secretsRoot != null) {
      yield r'secrets_root';
      yield serializers.serialize(
        object.secretsRoot,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.sshCredentialId != null) {
      yield r'ssh_credential_id';
      yield serializers.serialize(
        object.sshCredentialId,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
    if (object.trustedHostFingerprint != null) {
      yield r'trusted_host_fingerprint';
      yield serializers.serialize(
        object.trustedHostFingerprint,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'updated_at';
    yield serializers.serialize(
      object.updatedAt,
      specifiedType: const FullType(String),
    );
    if (object.username != null) {
      yield r'username';
      yield serializers.serialize(
        object.username,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'version';
    yield serializers.serialize(
      object.version,
      specifiedType: const FullType(int),
    );
    if (object.workRoot != null) {
      yield r'work_root';
      yield serializers.serialize(
        object.workRoot,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    NodeResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required NodeResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'checked_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.checkedAt = valueDes;
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
          break;
        case r'host':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.host = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
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
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.port = valueDes;
          break;
        case r'privileged_execution':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.privilegedExecution = valueDes;
          break;
        case r'secrets_root':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
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
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        case r'trusted_host_fingerprint':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.trustedHostFingerprint = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.updatedAt = valueDes;
          break;
        case r'username':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.username = valueDes;
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.version = valueDes;
          break;
        case r'work_root':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
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
  NodeResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = NodeResponseBuilder();
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
