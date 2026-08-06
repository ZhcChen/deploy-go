//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/git_credential_response.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'git_credential_list_response.g.dart';

/// GitCredentialListResponse
///
/// Properties:
/// * [items]
/// * [nextCursor]
@BuiltValue()
abstract class GitCredentialListResponse implements Built<GitCredentialListResponse, GitCredentialListResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<GitCredentialResponse> get items;

  @BuiltValueField(wireName: r'next_cursor')
  String? get nextCursor;

  GitCredentialListResponse._();

  factory GitCredentialListResponse([void updates(GitCredentialListResponseBuilder b)]) = _$GitCredentialListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(GitCredentialListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<GitCredentialListResponse> get serializer => _$GitCredentialListResponseSerializer();
}

class _$GitCredentialListResponseSerializer implements PrimitiveSerializer<GitCredentialListResponse> {
  @override
  final Iterable<Type> types = const [GitCredentialListResponse, _$GitCredentialListResponse];

  @override
  final String wireName = r'GitCredentialListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    GitCredentialListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(GitCredentialResponse)]),
    );
    if (object.nextCursor != null) {
      yield r'next_cursor';
      yield serializers.serialize(
        object.nextCursor,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    GitCredentialListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required GitCredentialListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(GitCredentialResponse)]),
          ) as BuiltList<GitCredentialResponse>;
          result.items.replace(valueDes);
          break;
        case r'next_cursor':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.nextCursor = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  GitCredentialListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = GitCredentialListResponseBuilder();
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
