//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/git_ref_response.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'git_ref_discovery_response.g.dart';

/// GitRefDiscoveryResponse
///
/// Properties:
/// * [applicationSourceId]
/// * [createdAt]
/// * [errorCode]
/// * [expiresAt]
/// * [finishedAt]
/// * [id]
/// * [refs]
/// * [sourceVersion]
/// * [status]
/// * [taskId]
@BuiltValue()
abstract class GitRefDiscoveryResponse implements Built<GitRefDiscoveryResponse, GitRefDiscoveryResponseBuilder> {
  @BuiltValueField(wireName: r'application_source_id')
  String get applicationSourceId;

  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'error_code')
  String? get errorCode;

  @BuiltValueField(wireName: r'expires_at')
  String? get expiresAt;

  @BuiltValueField(wireName: r'finished_at')
  String? get finishedAt;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'refs')
  BuiltList<GitRefResponse> get refs;

  @BuiltValueField(wireName: r'source_version')
  int get sourceVersion;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'task_id')
  String get taskId;

  GitRefDiscoveryResponse._();

  factory GitRefDiscoveryResponse([void updates(GitRefDiscoveryResponseBuilder b)]) = _$GitRefDiscoveryResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(GitRefDiscoveryResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<GitRefDiscoveryResponse> get serializer => _$GitRefDiscoveryResponseSerializer();
}

class _$GitRefDiscoveryResponseSerializer implements PrimitiveSerializer<GitRefDiscoveryResponse> {
  @override
  final Iterable<Type> types = const [GitRefDiscoveryResponse, _$GitRefDiscoveryResponse];

  @override
  final String wireName = r'GitRefDiscoveryResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    GitRefDiscoveryResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'application_source_id';
    yield serializers.serialize(
      object.applicationSourceId,
      specifiedType: const FullType(String),
    );
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
    if (object.errorCode != null) {
      yield r'error_code';
      yield serializers.serialize(
        object.errorCode,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.expiresAt != null) {
      yield r'expires_at';
      yield serializers.serialize(
        object.expiresAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.finishedAt != null) {
      yield r'finished_at';
      yield serializers.serialize(
        object.finishedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'refs';
    yield serializers.serialize(
      object.refs,
      specifiedType: const FullType(BuiltList, [FullType(GitRefResponse)]),
    );
    yield r'source_version';
    yield serializers.serialize(
      object.sourceVersion,
      specifiedType: const FullType(int),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
    yield r'task_id';
    yield serializers.serialize(
      object.taskId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    GitRefDiscoveryResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required GitRefDiscoveryResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'application_source_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.applicationSourceId = valueDes;
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
          break;
        case r'error_code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.errorCode = valueDes;
          break;
        case r'expires_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.expiresAt = valueDes;
          break;
        case r'finished_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.finishedAt = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'refs':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(GitRefResponse)]),
          ) as BuiltList<GitRefResponse>;
          result.refs.replace(valueDes);
          break;
        case r'source_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.sourceVersion = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        case r'task_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.taskId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  GitRefDiscoveryResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = GitRefDiscoveryResponseBuilder();
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
