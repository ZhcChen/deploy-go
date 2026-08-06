//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/secret_file_reference.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'save_target_request.g.dart';

/// SaveTargetRequest
///
/// Properties:
/// * [executionMode]
/// * [nodeId]
/// * [parameterSchema]
/// * [scriptPath]
/// * [secretFileReferences]
/// * [timeoutSeconds]
/// * [verificationConfig]
/// * [version]
@BuiltValue()
abstract class SaveTargetRequest implements Built<SaveTargetRequest, SaveTargetRequestBuilder> {
  @BuiltValueField(wireName: r'execution_mode')
  String? get executionMode;

  @BuiltValueField(wireName: r'node_id')
  String get nodeId;

  @BuiltValueField(wireName: r'parameter_schema')
  JsonObject? get parameterSchema;

  @BuiltValueField(wireName: r'script_path')
  String get scriptPath;

  @BuiltValueField(wireName: r'secret_file_references')
  BuiltList<SecretFileReference>? get secretFileReferences;

  @BuiltValueField(wireName: r'timeout_seconds')
  int get timeoutSeconds;

  @BuiltValueField(wireName: r'verification_config')
  JsonObject? get verificationConfig;

  @BuiltValueField(wireName: r'version')
  int? get version;

  SaveTargetRequest._();

  factory SaveTargetRequest([void updates(SaveTargetRequestBuilder b)]) = _$SaveTargetRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SaveTargetRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SaveTargetRequest> get serializer => _$SaveTargetRequestSerializer();
}

class _$SaveTargetRequestSerializer implements PrimitiveSerializer<SaveTargetRequest> {
  @override
  final Iterable<Type> types = const [SaveTargetRequest, _$SaveTargetRequest];

  @override
  final String wireName = r'SaveTargetRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SaveTargetRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.executionMode != null) {
      yield r'execution_mode';
      yield serializers.serialize(
        object.executionMode,
        specifiedType: const FullType(String),
      );
    }
    yield r'node_id';
    yield serializers.serialize(
      object.nodeId,
      specifiedType: const FullType(String),
    );
    yield r'parameter_schema';
    yield object.parameterSchema == null ? null : serializers.serialize(
      object.parameterSchema,
      specifiedType: const FullType.nullable(JsonObject),
    );
    yield r'script_path';
    yield serializers.serialize(
      object.scriptPath,
      specifiedType: const FullType(String),
    );
    if (object.secretFileReferences != null) {
      yield r'secret_file_references';
      yield serializers.serialize(
        object.secretFileReferences,
        specifiedType: const FullType(BuiltList, [FullType(SecretFileReference)]),
      );
    }
    yield r'timeout_seconds';
    yield serializers.serialize(
      object.timeoutSeconds,
      specifiedType: const FullType(int),
    );
    yield r'verification_config';
    yield object.verificationConfig == null ? null : serializers.serialize(
      object.verificationConfig,
      specifiedType: const FullType.nullable(JsonObject),
    );
    if (object.version != null) {
      yield r'version';
      yield serializers.serialize(
        object.version,
        specifiedType: const FullType.nullable(int),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    SaveTargetRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SaveTargetRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'execution_mode':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.executionMode = valueDes;
          break;
        case r'node_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.nodeId = valueDes;
          break;
        case r'parameter_schema':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.parameterSchema = valueDes;
          break;
        case r'script_path':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.scriptPath = valueDes;
          break;
        case r'secret_file_references':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(BuiltList, [FullType(SecretFileReference)]),
          ) as BuiltList<SecretFileReference>?;
          if (valueDes == null) continue;
          result.secretFileReferences.replace(valueDes);
          break;
        case r'timeout_seconds':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.timeoutSeconds = valueDes;
          break;
        case r'verification_config':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.verificationConfig = valueDes;
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.version = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SaveTargetRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SaveTargetRequestBuilder();
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
