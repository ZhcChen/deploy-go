//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'save_source_request.g.dart';

/// SaveSourceRequest
///
/// Properties:
/// * [buildAgentId]
/// * [gitCredentialId]
/// * [repositoryUrl]
/// * [sourcePolicy]
/// * [version]
@BuiltValue()
abstract class SaveSourceRequest implements Built<SaveSourceRequest, SaveSourceRequestBuilder> {
  @BuiltValueField(wireName: r'build_agent_id')
  String get buildAgentId;

  @BuiltValueField(wireName: r'git_credential_id')
  String? get gitCredentialId;

  @BuiltValueField(wireName: r'repository_url')
  String get repositoryUrl;

  @BuiltValueField(wireName: r'source_policy')
  String? get sourcePolicy;

  @BuiltValueField(wireName: r'version')
  int? get version;

  SaveSourceRequest._();

  factory SaveSourceRequest([void updates(SaveSourceRequestBuilder b)]) = _$SaveSourceRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SaveSourceRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SaveSourceRequest> get serializer => _$SaveSourceRequestSerializer();
}

class _$SaveSourceRequestSerializer implements PrimitiveSerializer<SaveSourceRequest> {
  @override
  final Iterable<Type> types = const [SaveSourceRequest, _$SaveSourceRequest];

  @override
  final String wireName = r'SaveSourceRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SaveSourceRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'build_agent_id';
    yield serializers.serialize(
      object.buildAgentId,
      specifiedType: const FullType(String),
    );
    if (object.gitCredentialId != null) {
      yield r'git_credential_id';
      yield serializers.serialize(
        object.gitCredentialId,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'repository_url';
    yield serializers.serialize(
      object.repositoryUrl,
      specifiedType: const FullType(String),
    );
    if (object.sourcePolicy != null) {
      yield r'source_policy';
      yield serializers.serialize(
        object.sourcePolicy,
        specifiedType: const FullType(String),
      );
    }
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
    SaveSourceRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SaveSourceRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'build_agent_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.buildAgentId = valueDes;
          break;
        case r'git_credential_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.gitCredentialId = valueDes;
          break;
        case r'repository_url':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.repositoryUrl = valueDes;
          break;
        case r'source_policy':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.sourcePolicy = valueDes;
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
  SaveSourceRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SaveSourceRequestBuilder();
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
